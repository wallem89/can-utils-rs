use anyhow::Result;
use anyhow::bail;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
use std::io::{self, Stdout};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::dump::{detect_can_interfaces, format_frame};
use crate::setup::exec::{
    execute_bring_up, execute_config, interface_exists, remove_existing_interface,
};
use crate::setup::models::{
    CanBitrate, CanConfig, CanMode, NativeConfig, SlcanConfig, slcan_speeds,
};
use crate::setup::plan::plan_lines;
use crate::setup::prereqs::{
    Prerequisite, has_apt, install_missing_prerequisites_without_prompt, missing_prerequisites,
    packages_for_missing_prerequisites,
};
use crate::setup::prompt::list_serial_candidates;
use crate::{ToolAction, run_action};

const UART_BAUD_OPTIONS: [&str; 5] = ["115200", "230400", "460800", "921600", "3000000"];

pub fn run() -> Result<()> {
    let mut terminal = TerminalSession::enter()?;
    let mut app = App::default();

    loop {
        terminal.draw(|frame| app.render(frame))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.handle_key(key.code, &mut terminal)? {
            return Ok(());
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, draw_fn: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame<'_>),
    {
        self.terminal.draw(draw_fn)?;
        Ok(())
    }

    fn suspend(&mut self) -> Result<()> {
        disable_raw_mode()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        enable_raw_mode()?;
        Ok(())
    }

    fn run_with_terminal<F, T>(&mut self, action: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        self.suspend()?;
        let result = action();
        let resume_result = self.resume();

        match (result, resume_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), Ok(())) => Err(err),
            (Ok(_), Err(resume_err)) => Err(resume_err.into()),
            (Err(err), Err(_)) => Err(err),
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

struct App {
    selected: usize,
    status: String,
    setup: Option<SetupFlow>,
    dump: Option<DumpFlow>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selected: 0,
            status: "Press Enter to run an action, q to quit.".to_string(),
            setup: None,
            dump: None,
        }
    }
}

impl App {
    fn actions() -> [ToolAction; 4] {
        [
            ToolAction::Setup,
            ToolAction::Dump,
            ToolAction::SetupAndDump,
            ToolAction::Send,
        ]
    }

    fn next(&mut self) {
        self.selected = (self.selected + 1) % Self::actions().len();
    }

    fn previous(&mut self) {
        self.selected = if self.selected == 0 {
            Self::actions().len() - 1
        } else {
            self.selected - 1
        };
    }

    fn selected_action(&self) -> ToolAction {
        Self::actions()[self.selected]
    }

    fn handle_key(&mut self, key: KeyCode, terminal: &mut TerminalSession) -> Result<bool> {
        if self.dump.is_some() {
            let should_exit_dump = self.handle_dump_key(key)?;
            if should_exit_dump {
                self.dump = None;
            }
            return Ok(false);
        }

        if self.setup.is_some() {
            return self
                .handle_setup_key(key, terminal)
                .map(|should_exit_setup| {
                    if should_exit_setup {
                        self.setup = None;
                    }
                    false
                });
        }

        match key {
            KeyCode::Up | KeyCode::Char('k') => self.previous(),
            KeyCode::Down | KeyCode::Char('j') => self.next(),
            KeyCode::Enter => {
                let action = self.selected_action();

                if matches!(action, ToolAction::Setup | ToolAction::SetupAndDump) {
                    self.setup = Some(SetupFlow::new(matches!(action, ToolAction::SetupAndDump)));
                } else if matches!(action, ToolAction::Dump) {
                    self.dump = Some(DumpFlow::new()?);
                } else {
                    let result = terminal.run_with_terminal(|| run_action(action));
                    match result {
                        Ok(()) => self.status = format!("Finished: {}", action),
                        Err(err) => self.status = format!("Last action failed: {err:#}"),
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
            _ => {}
        }

        Ok(false)
    }

    fn handle_setup_key(&mut self, key: KeyCode, terminal: &mut TerminalSession) -> Result<bool> {
        let mut flow = self.setup.take().expect("setup flow should exist");
        let transition = flow.handle_key(key, terminal, &mut self.status)?;

        match transition {
            SetupTransition::Stay => {
                self.setup = Some(flow);
            }
            SetupTransition::Exit => {}
            SetupTransition::StartDump(iface) => {
                self.dump = Some(DumpFlow::with_interface(iface)?);
            }
        }

        Ok(false)
    }

    fn handle_dump_key(&mut self, key: KeyCode) -> Result<bool> {
        let mut flow = self.dump.take().expect("dump flow should exist");
        let exit_dump = flow.handle_key(key, &mut self.status)?;

        if !exit_dump {
            flow.poll_frames(&mut self.status)?;
            self.dump = Some(flow);
        }

        Ok(exit_dump)
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>) {
        if let Some(dump) = &self.dump {
            dump.render(frame, &self.status);
            return;
        }

        if let Some(setup) = &self.setup {
            setup.render(frame, &self.status);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(frame.area());

        let title = Paragraph::new("can-utils-rs")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Menu"));
        frame.render_widget(title, chunks[0]);

        let items = Self::actions()
            .into_iter()
            .map(|action| ListItem::new(Line::from(action.to_string())))
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.selected));
        frame.render_stateful_widget(list, chunks[1], &mut state);

        let status = Paragraph::new(Text::from(self.status.clone()))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, chunks[2]);
    }
}

struct SetupFlow {
    after_setup_dump: bool,
    step: SetupStep,
    draft: SetupDraft,
    prereq_selected: usize,
    form_field: usize,
    conflict_selected: usize,
    plan_selected: usize,
    replace_existing: bool,
}

impl SetupFlow {
    fn new(after_setup_dump: bool) -> Self {
        let missing = missing_prerequisites();
        let step = if missing.is_empty() {
            SetupStep::Mode { selected: 0 }
        } else {
            SetupStep::Prereqs { missing }
        };

        Self {
            after_setup_dump,
            step,
            draft: SetupDraft::default(),
            prereq_selected: 0,
            form_field: 0,
            conflict_selected: 0,
            plan_selected: 0,
            replace_existing: false,
        }
    }

    fn handle_key(
        &mut self,
        key: KeyCode,
        terminal: &mut TerminalSession,
        status: &mut String,
    ) -> Result<SetupTransition> {
        match &mut self.step {
            SetupStep::Prereqs { missing } => {
                let action_count = if has_apt() { 3 } else { 2 };
                match key {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.prereq_selected = self.prereq_selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.prereq_selected = (self.prereq_selected + 1).min(action_count - 1);
                    }
                    KeyCode::Enter => {
                        if has_apt() && self.prereq_selected == 0 {
                            let result = terminal.run_with_terminal(|| {
                                install_missing_prerequisites_without_prompt(missing)
                            });

                            match result {
                                Ok(()) => {
                                    let still_missing = missing_prerequisites();
                                    if still_missing.is_empty() {
                                        *status = "Installed missing prerequisites.".to_string();
                                        self.step = SetupStep::Mode { selected: 0 };
                                    } else {
                                        *status = format!(
                                            "Some prerequisites are still missing: {}",
                                            join_prerequisites(&still_missing)
                                        );
                                        self.step = SetupStep::Prereqs {
                                            missing: still_missing,
                                        };
                                    }
                                }
                                Err(err) => {
                                    *status = format!("Prerequisite installation failed: {err:#}");
                                }
                            }
                        } else if self.prereq_selected == if has_apt() { 1 } else { 0 } {
                            self.step = SetupStep::Mode { selected: 0 };
                        } else {
                            *status = "Cancelled setup and returned to the menu.".to_string();
                            return Ok(SetupTransition::Exit);
                        }
                    }
                    KeyCode::Esc => {
                        *status = "Cancelled setup and returned to the menu.".to_string();
                        return Ok(SetupTransition::Exit);
                    }
                    _ => {}
                }
            }
            SetupStep::Mode { selected } => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1).min(2);
                }
                KeyCode::Enter => {
                    self.draft.set_mode(*selected);
                    self.form_field = 0;
                    self.replace_existing = false;
                    self.step = SetupStep::Form;
                }
                KeyCode::Esc => {
                    *status = "Cancelled setup and returned to the menu.".to_string();
                    return Ok(SetupTransition::Exit);
                }
                _ => {}
            },
            SetupStep::Form => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.form_field = self.form_field.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                    self.form_field = (self.form_field + 1).min(self.draft.form_len() - 1);
                }
                KeyCode::Left | KeyCode::Char('h') => self.draft.cycle_option(self.form_field, -1),
                KeyCode::Right | KeyCode::Char('l') => self.draft.cycle_option(self.form_field, 1),
                KeyCode::Backspace => self.draft.backspace(self.form_field),
                KeyCode::Char(ch) => self.draft.push_char(self.form_field, ch),
                KeyCode::Enter => {
                    let config = match self.draft.build_config() {
                        Ok(config) => config,
                        Err(err) => {
                            *status = err.to_string();
                            return Ok(SetupTransition::Stay);
                        }
                    };

                    if interface_exists(config.iface()) {
                        self.conflict_selected = 0;
                        self.step = SetupStep::Conflict { config };
                    } else {
                        self.plan_selected = 0;
                        self.step = SetupStep::Plan { config };
                    }
                }
                KeyCode::Esc => {
                    self.step = SetupStep::Mode {
                        selected: self.draft.mode_index(),
                    };
                }
                _ => {}
            },
            SetupStep::Conflict { config } => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.conflict_selected = self.conflict_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.conflict_selected = (self.conflict_selected + 1).min(3);
                }
                KeyCode::Enter => match self.conflict_selected {
                    0 => {
                        self.replace_existing = true;
                        self.plan_selected = 0;
                        self.step = SetupStep::Plan {
                            config: config.clone(),
                        };
                    }
                    1 => {
                        let iface = config.iface().to_string();
                        self.draft.copy_iface_from_config(config);
                        self.form_field = self.draft.iface_field_index();
                        self.step = SetupStep::Form;
                        *status = format!(
                            "Edit the interface name for '{}' and press Enter again.",
                            iface
                        );
                    }
                    2 => {
                        let bring_up_config = config.clone();
                        let result =
                            terminal.run_with_terminal(|| execute_bring_up(&bring_up_config));

                        match result {
                            Ok(()) => {
                                *status = format!(
                                    "Kept existing interface '{}' and skipped setup.",
                                    bring_up_config.iface()
                                );
                                if self.after_setup_dump {
                                    return Ok(SetupTransition::StartDump(
                                        bring_up_config.iface().to_string(),
                                    ));
                                }
                                return Ok(SetupTransition::Exit);
                            }
                            Err(err) => {
                                *status = format!("Failed while reusing interface: {err:#}");
                                return Ok(SetupTransition::Exit);
                            }
                        }
                    }
                    _ => {
                        *status = "Cancelled setup and returned to the menu.".to_string();
                        return Ok(SetupTransition::Exit);
                    }
                },
                KeyCode::Esc => {
                    self.step = SetupStep::Form;
                }
                _ => {}
            },
            SetupStep::Plan { config } => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.plan_selected = self.plan_selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.plan_selected = (self.plan_selected + 1).min(2);
                }
                KeyCode::Enter => match self.plan_selected {
                    0 => {
                        let config_to_run = config.clone();
                        let replace_existing = self.replace_existing;
                        let result = terminal.run_with_terminal(|| {
                            if replace_existing {
                                remove_existing_interface(&config_to_run)?;
                            }
                            execute_config(&config_to_run)?;
                            Ok(())
                        });

                        match result {
                            Ok(()) => {
                                *status = format!(
                                    "Set up interface '{}' successfully.",
                                    config_to_run.iface()
                                );
                                if self.after_setup_dump {
                                    return Ok(SetupTransition::StartDump(
                                        config_to_run.iface().to_string(),
                                    ));
                                }
                            }
                            Err(err) => {
                                *status = format!("Setup failed: {err:#}");
                            }
                        }

                        return Ok(SetupTransition::Exit);
                    }
                    1 => {
                        self.step = SetupStep::Form;
                    }
                    _ => {
                        *status = "Cancelled setup and returned to the menu.".to_string();
                        return Ok(SetupTransition::Exit);
                    }
                },
                KeyCode::Esc => {
                    self.step = SetupStep::Form;
                }
                _ => {}
            },
        }

        Ok(SetupTransition::Stay)
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, status: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(12),
                Constraint::Length(4),
            ])
            .split(frame.area());

        let title = if self.after_setup_dump {
            "Setup CAN Interface And Dump"
        } else {
            "Setup CAN Interface"
        };

        let header = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Wizard"));
        frame.render_widget(header, chunks[0]);

        match &self.step {
            SetupStep::Prereqs { missing } => self.render_prereqs(frame, chunks[1], missing),
            SetupStep::Mode { selected } => self.render_mode(frame, chunks[1], *selected),
            SetupStep::Form => self.render_form(frame, chunks[1]),
            SetupStep::Conflict { config } => self.render_conflict(frame, chunks[1], config),
            SetupStep::Plan { config } => self.render_plan(frame, chunks[1], config),
        }

        let footer = Paragraph::new(Text::from(format!("{}\n{}", self.help_text(), status)))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(footer, chunks[2]);
    }

    fn help_text(&self) -> &'static str {
        match self.step {
            SetupStep::Prereqs { .. } => {
                "Use arrows to choose an action. Enter applies it. Esc returns to the menu."
            }
            SetupStep::Mode { .. } => {
                "Choose a CAN connection type. Enter continues. Esc returns to the menu."
            }
            SetupStep::Form => {
                "Use arrows to move fields. Type to edit text. Left/right changes selections. Enter previews commands."
            }
            SetupStep::Conflict { .. } => {
                "Choose how to handle the existing interface. Enter applies it. Esc returns to editing."
            }
            SetupStep::Plan { .. } => {
                "Review the commands. Enter executes or goes back. Esc returns to editing."
            }
        }
    }

    fn render_prereqs(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        missing: &[Prerequisite],
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(6)])
            .split(area);

        let details = if has_apt() {
            match packages_for_missing_prerequisites(missing) {
                Ok(packages) => format!(
                    "Missing prerequisites: {}\nPackages to install: {}",
                    join_prerequisites(missing),
                    packages.join(", ")
                ),
                Err(_) => format!("Missing prerequisites: {}", join_prerequisites(missing)),
            }
        } else {
            format!("Missing prerequisites: {}", join_prerequisites(missing))
        };

        let missing_block = Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Prerequisites"),
        );
        frame.render_widget(missing_block, layout[0]);

        let mut actions = Vec::new();
        if has_apt() {
            actions.push(ListItem::new("Install missing prerequisites"));
        }
        actions.push(ListItem::new("Continue anyway"));
        actions.push(ListItem::new("Back to menu"));

        let list = List::new(actions)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.prereq_selected));
        frame.render_stateful_widget(list, layout[1], &mut state);
    }

    fn render_mode(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        selected: usize,
    ) {
        let items = [CanMode::Native, CanMode::Slcan, CanMode::Virtual]
            .into_iter()
            .map(|mode| ListItem::new(mode.to_string()))
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select CAN connection type"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(selected));
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn render_form(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(4)])
            .split(area);

        let fields = self.draft.form_lines();
        let items = fields
            .into_iter()
            .enumerate()
            .map(|(idx, line)| {
                let content = if idx == self.form_field {
                    format!("> {line}")
                } else {
                    format!("  {line}")
                };
                ListItem::new(content)
            })
            .collect::<Vec<_>>();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.draft.form_title()),
        );
        frame.render_widget(list, layout[0]);

        let hint = if matches!(self.draft.mode, CanMode::Slcan)
            && !self.draft.serial_candidates.is_empty()
        {
            format!(
                "Detected serial devices: {}",
                self.draft.serial_candidates.join(", ")
            )
        } else {
            "Press Enter to preview the commands for this setup.".to_string()
        };

        let preview =
            Paragraph::new(hint).block(Block::default().borders(Borders::ALL).title("Hint"));
        frame.render_widget(preview, layout[1]);
    }

    fn render_conflict(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        config: &CanConfig,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(8)])
            .split(area);

        let info = Paragraph::new(format!(
            "Interface '{}' already exists. Choose whether to replace it, rename the new interface, or keep the current one.",
            config.iface()
        ))
        .block(Block::default().borders(Borders::ALL).title("Existing Interface"));
        frame.render_widget(info, layout[0]);

        let items = vec![
            ListItem::new("Replace existing interface"),
            ListItem::new("Edit interface name"),
            ListItem::new("Keep existing and skip setup"),
            ListItem::new("Back to menu"),
        ];

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.conflict_selected));
        frame.render_stateful_widget(list, layout[1], &mut state);
    }

    fn render_plan(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        config: &CanConfig,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(6)])
            .split(area);

        let mut lines = Vec::new();
        if self.replace_existing {
            lines.push("Existing interface will be removed first.".to_string());
        }
        lines.extend(plan_lines(config));

        let commands = Paragraph::new(lines.join("\n")).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Planned Commands"),
        );
        frame.render_widget(commands, layout[0]);

        let action_items = vec![
            ListItem::new(if self.after_setup_dump {
                "Execute now and start dump"
            } else {
                "Execute now"
            }),
            ListItem::new("Back to edit"),
            ListItem::new("Back to menu"),
        ];

        let list = List::new(action_items)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.plan_selected));
        frame.render_stateful_widget(list, layout[1], &mut state);
    }
}

enum SetupStep {
    Prereqs { missing: Vec<Prerequisite> },
    Mode { selected: usize },
    Form,
    Conflict { config: CanConfig },
    Plan { config: CanConfig },
}

enum SetupTransition {
    Stay,
    Exit,
    StartDump(String),
}

struct DumpFlow {
    step: DumpStep,
    selected: usize,
    manual_iface: String,
    frames: Vec<String>,
}

impl DumpFlow {
    fn new() -> Result<Self> {
        let mut ifaces = detect_can_interfaces()?;
        ifaces.push("Enter manually".to_string());

        Ok(Self {
            step: DumpStep::Select { ifaces },
            selected: 0,
            manual_iface: String::new(),
            frames: Vec::new(),
        })
    }

    fn with_interface(iface: String) -> Result<Self> {
        let mut flow = Self {
            step: DumpStep::Select { ifaces: Vec::new() },
            selected: 0,
            manual_iface: iface.clone(),
            frames: Vec::new(),
        };
        flow.start_stream(iface)?;
        Ok(flow)
    }

    fn handle_key(&mut self, key: KeyCode, status: &mut String) -> Result<bool> {
        match &mut self.step {
            DumpStep::Select { ifaces } => match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.selected = self.selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected = (self.selected + 1).min(ifaces.len().saturating_sub(1));
                }
                KeyCode::Enter => {
                    let iface = if ifaces.get(self.selected).map(String::as_str)
                        == Some("Enter manually")
                    {
                        self.manual_iface.trim().to_string()
                    } else {
                        ifaces[self.selected].clone()
                    };

                    let iface = required_text("CAN interface name", &iface)?;
                    self.start_stream(iface.clone())?;
                    *status = format!("Started CAN dump on '{iface}'.");
                }
                KeyCode::Backspace => {
                    if ifaces.get(self.selected).map(String::as_str) == Some("Enter manually") {
                        self.manual_iface.pop();
                    }
                }
                KeyCode::Char(ch) => {
                    if ifaces.get(self.selected).map(String::as_str) == Some("Enter manually")
                        && !ch.is_control()
                    {
                        self.manual_iface.push(ch);
                    }
                }
                KeyCode::Esc => {
                    *status = "Cancelled dump and returned to the menu.".to_string();
                    return Ok(true);
                }
                _ => {}
            },
            DumpStep::Stream { .. } => match key {
                KeyCode::Esc | KeyCode::Char('q') => {
                    *status = "Stopped CAN dump and returned to the menu.".to_string();
                    return Ok(true);
                }
                _ => {}
            },
        }

        Ok(false)
    }

    fn poll_frames(&mut self, status: &mut String) -> Result<()> {
        let DumpStep::Stream { iface, socket } = &mut self.step else {
            return Ok(());
        };

        for _ in 0..64 {
            match socket.read_frame_timeout(Duration::from_millis(1)) {
                Ok(frame) => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("system time before unix epoch");

                    let ts = format!("({}.{:06})", now.as_secs(), now.subsec_micros());
                    let line = format_frame(&ts, iface, frame.raw_id(), frame.data());
                    self.frames.push(line);

                    if self.frames.len() > 500 {
                        let extra = self.frames.len() - 500;
                        self.frames.drain(0..extra);
                    }
                }
                Err(err)
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.kind() == io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(err) => {
                    *status = format!("CAN dump failed: {err}");
                    return Err(err.into());
                }
            }
        }

        Ok(())
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, status: &str) {
        match &self.step {
            DumpStep::Select { ifaces } => self.render_select(frame, status, ifaces),
            DumpStep::Stream { iface, .. } => self.render_stream(frame, status, iface),
        }
    }

    fn render_select(&self, frame: &mut ratatui::Frame<'_>, status: &str, ifaces: &[String]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .split(frame.area());

        let header = Paragraph::new("Pretty CAN Dump")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Dump"));
        frame.render_widget(header, chunks[0]);

        let items = ifaces
            .iter()
            .map(|iface| {
                if iface == "Enter manually" {
                    ListItem::new(format!("{iface}: {}", self.manual_iface))
                } else {
                    ListItem::new(iface.clone())
                }
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select CAN interface to dump"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        state.select(Some(self.selected));
        frame.render_stateful_widget(list, chunks[1], &mut state);

        let footer = Paragraph::new(Text::from(format!(
            "Use arrows to choose an interface. Type when 'Enter manually' is selected. Enter starts the live dump.\n{}",
            status
        )))
        .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(footer, chunks[2]);
    }

    fn render_stream(&self, frame: &mut ratatui::Frame<'_>, status: &str, iface: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .split(frame.area());

        let header = Paragraph::new(format!("Pretty CAN Dump on {iface}"))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Live"));
        frame.render_widget(header, chunks[0]);

        let height = chunks[1].height.saturating_sub(2) as usize;
        let start = self.frames.len().saturating_sub(height);
        let lines = if self.frames.is_empty() {
            "Waiting for CAN frames...".to_string()
        } else {
            self.frames[start..].join("\n")
        };

        let body =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Frames"));
        frame.render_widget(body, chunks[1]);

        let footer = Paragraph::new(Text::from(format!(
            "Live dump is running inside ratatui. Press q or Esc to return to the menu.\n{}",
            status
        )))
        .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(footer, chunks[2]);
    }

    fn start_stream(&mut self, iface: String) -> Result<()> {
        let socket = CanSocket::open(&iface)?;
        self.frames.clear();
        self.step = DumpStep::Stream { iface, socket };
        Ok(())
    }
}

enum DumpStep {
    Select { ifaces: Vec<String> },
    Stream { iface: String, socket: CanSocket },
}

struct SetupDraft {
    mode: CanMode,
    native_iface: String,
    native_bitrate: usize,
    slcan_tty: String,
    slcan_iface: String,
    slcan_speed: usize,
    slcan_uart: usize,
    virtual_iface: String,
    serial_candidates: Vec<String>,
}

impl Default for SetupDraft {
    fn default() -> Self {
        Self {
            mode: CanMode::Native,
            native_iface: "can0".to_string(),
            native_bitrate: 1,
            slcan_tty: "/dev/ttyUSB0".to_string(),
            slcan_iface: "slcan0".to_string(),
            slcan_speed: 2,
            slcan_uart: 4,
            virtual_iface: "vcan0".to_string(),
            serial_candidates: list_serial_candidates().unwrap_or_default(),
        }
    }
}

impl SetupDraft {
    fn set_mode(&mut self, selected: usize) {
        self.mode = match selected {
            0 => CanMode::Native,
            1 => CanMode::Slcan,
            _ => CanMode::Virtual,
        };
    }

    fn mode_index(&self) -> usize {
        match self.mode {
            CanMode::Native => 0,
            CanMode::Slcan => 1,
            CanMode::Virtual => 2,
        }
    }

    fn form_title(&self) -> &'static str {
        match self.mode {
            CanMode::Native => "Native CAN Configuration",
            CanMode::Slcan => "SLCAN Configuration",
            CanMode::Virtual => "Virtual CAN Configuration",
        }
    }

    fn form_len(&self) -> usize {
        match self.mode {
            CanMode::Native => 2,
            CanMode::Slcan => 4,
            CanMode::Virtual => 1,
        }
    }

    fn iface_field_index(&self) -> usize {
        match self.mode {
            CanMode::Native => 0,
            CanMode::Slcan => 1,
            CanMode::Virtual => 0,
        }
    }

    fn form_lines(&self) -> Vec<String> {
        match self.mode {
            CanMode::Native => {
                let bitrates = CanBitrate::can_bitrates();
                vec![
                    format!("Interface name: {}", self.native_iface),
                    format!("CAN bitrate: {}", bitrates[self.native_bitrate]),
                ]
            }
            CanMode::Slcan => {
                let speeds = slcan_speeds();
                vec![
                    format!("Serial device: {}", self.slcan_tty),
                    format!("Interface name: {}", self.slcan_iface),
                    format!("CAN bitrate: {}", speeds[self.slcan_speed]),
                    format!("UART baud rate: {}", UART_BAUD_OPTIONS[self.slcan_uart]),
                ]
            }
            CanMode::Virtual => vec![format!("Interface name: {}", self.virtual_iface)],
        }
    }

    fn cycle_option(&mut self, field: usize, delta: isize) {
        match self.mode {
            CanMode::Native if field == 1 => {
                self.native_bitrate =
                    cycle_index(self.native_bitrate, CanBitrate::can_bitrates().len(), delta);
            }
            CanMode::Slcan if field == 2 => {
                self.slcan_speed = cycle_index(self.slcan_speed, slcan_speeds().len(), delta);
            }
            CanMode::Slcan if field == 3 => {
                self.slcan_uart = cycle_index(self.slcan_uart, UART_BAUD_OPTIONS.len(), delta);
            }
            _ => {}
        }
    }

    fn push_char(&mut self, field: usize, ch: char) {
        if ch.is_control() {
            return;
        }

        match self.mode {
            CanMode::Native if field == 0 => self.native_iface.push(ch),
            CanMode::Slcan if field == 0 => self.slcan_tty.push(ch),
            CanMode::Slcan if field == 1 => self.slcan_iface.push(ch),
            CanMode::Virtual if field == 0 => self.virtual_iface.push(ch),
            _ => {}
        }
    }

    fn backspace(&mut self, field: usize) {
        match self.mode {
            CanMode::Native if field == 0 => {
                self.native_iface.pop();
            }
            CanMode::Slcan if field == 0 => {
                self.slcan_tty.pop();
            }
            CanMode::Slcan if field == 1 => {
                self.slcan_iface.pop();
            }
            CanMode::Virtual if field == 0 => {
                self.virtual_iface.pop();
            }
            _ => {}
        }
    }

    fn copy_iface_from_config(&mut self, config: &CanConfig) {
        match config {
            CanConfig::Native(cfg) => self.native_iface = cfg.iface.clone(),
            CanConfig::Slcan(cfg) => self.slcan_iface = cfg.iface.clone(),
            CanConfig::Virtual(cfg) => self.virtual_iface = cfg.iface.clone(),
        }
    }

    fn build_config(&self) -> Result<CanConfig> {
        match self.mode {
            CanMode::Native => Ok(CanConfig::Native(NativeConfig::new(
                required_text("CAN interface name", &self.native_iface)?,
                CanBitrate::can_bitrates()[self.native_bitrate],
            ))),
            CanMode::Slcan => Ok(CanConfig::Slcan(SlcanConfig::new(
                required_text("serial device", &self.slcan_tty)?,
                required_text("SLCAN interface name", &self.slcan_iface)?,
                slcan_speeds()[self.slcan_speed].clone(),
                UART_BAUD_OPTIONS[self.slcan_uart].parse()?,
            ))),
            CanMode::Virtual => Ok(CanConfig::Virtual(
                crate::setup::models::VirtualConfig::new(required_text(
                    "virtual interface name",
                    &self.virtual_iface,
                )?),
            )),
        }
    }
}

fn cycle_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }

    let next = current as isize + delta;
    next.rem_euclid(len as isize) as usize
}

fn required_text(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(trimmed.to_string())
}

fn join_prerequisites(missing: &[Prerequisite]) -> String {
    missing
        .iter()
        .map(|item| item.description())
        .collect::<Vec<_>>()
        .join(", ")
}
