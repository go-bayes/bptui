use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const EDITABLE_FIELDS: &[&str] =
    &["name", "description", "reference", "waves", "keywords", "standardised", "standardised_date", "items"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableField {
    Name,
    Description,
    Reference,
    Waves,
    Keywords,
    Standardised,
    StandardisedDate,
    Items,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Measures,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Insert,
    Normal,
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Insert
    }
}

impl FocusedPanel {
    pub fn next(self) -> Self {
        match self {
            Self::Measures => Self::Details,
            Self::Details => Self::Measures,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

const EDITABLE_MENU_FIELDS: [EditableField; 8] = [
    EditableField::Name,
    EditableField::Description,
    EditableField::Reference,
    EditableField::Waves,
    EditableField::Keywords,
    EditableField::Standardised,
    EditableField::StandardisedDate,
    EditableField::Items,
];

impl EditableField {
    fn key(self) -> &'static str {
        match self {
            EditableField::Name => "name",
            EditableField::Description => "description",
            EditableField::Reference => "reference",
            EditableField::Waves => "waves",
            EditableField::Keywords => "keywords",
            EditableField::Standardised => "standardised",
            EditableField::StandardisedDate => "standardised_date",
            EditableField::Items => "items",
        }
    }

    fn label(self) -> &'static str {
        match self {
            EditableField::Name => "Name",
            EditableField::Description => "Description",
            EditableField::Reference => "Reference",
            EditableField::Waves => "Waves",
            EditableField::Keywords => "Keywords",
            EditableField::Standardised => "Standardised",
            EditableField::StandardisedDate => "Standardised date",
            EditableField::Items => "Items",
        }
    }

    pub fn is_toggleable(self) -> bool {
        matches!(self, EditableField::Standardised)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MeasureRecord {
    pub name: String,
    pub description: Option<String>,
    pub reference: Option<String>,
    pub waves: Option<String>,
    pub keywords: Option<String>,
    pub standardised: Option<bool>,
    pub standardised_date: Option<String>,
    pub items: Vec<String>,
}

impl MeasureRecord {
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    fn value_is_present(value: &Option<String>) -> bool {
        value
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn validation(&self) -> MeasureValidation {
        MeasureValidation {
            description: Self::value_is_present(&self.description),
            reference: Self::value_is_present(&self.reference),
            waves: Self::value_is_present(&self.waves),
            keywords: Self::value_is_present(&self.keywords),
            standardised: self.standardised.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MeasureValidation {
    pub description: bool,
    pub reference: bool,
    pub waves: bool,
    pub keywords: bool,
    pub standardised: bool,
}

impl MeasureValidation {
    pub fn is_valid(&self) -> bool {
        self.description && self.reference && self.waves && self.keywords && self.standardised
    }
}

#[derive(Debug, Clone)]
struct MeasureHistoryEntry {
    id: usize,
    action: String,
    selected_measure: usize,
    measures: Vec<MeasureRecord>,
}

#[derive(Debug)]
struct CommandSpec {
    name: &'static str,
    arg_hint: Option<&'static str>,
    description: &'static str,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "help",
        arg_hint: None,
        description: "show help overlay",
    },
    CommandSpec {
        name: "save",
        arg_hint: None,
        description: "save current working set",
    },
    CommandSpec {
        name: "quit",
        arg_hint: None,
        description: "exit the tui",
    },
    CommandSpec {
        name: "q",
        arg_hint: None,
        description: "alias for /quit",
    },
    CommandSpec {
        name: "add",
        arg_hint: Some("<path>"),
        description: "add a measure path",
    },
    CommandSpec {
        name: "reload",
        arg_hint: None,
        description: "reload stub",
    },
    CommandSpec {
        name: "clear",
        arg_hint: None,
        description: "clear status text",
    },
    CommandSpec {
        name: "find",
        arg_hint: Some("<query>"),
        description: "select next matching measure",
    },
    CommandSpec {
        name: "edit",
        arg_hint: Some("<field> <value>"),
        description: "edit selected measure",
    },
    CommandSpec {
        name: "history",
        arg_hint: None,
        description: "list recent save checkpoints",
    },
    CommandSpec {
        name: "restore",
        arg_hint: Some("<index>"),
        description: "restore a checkpoint by list index",
    },
    CommandSpec {
        name: "revert",
        arg_hint: None,
        description: "revert to previous checkpoint",
    },
    CommandSpec {
        name: "undo",
        arg_hint: None,
        description: "alias for /revert",
    },
    CommandSpec {
        name: "e",
        arg_hint: None,
        description: "open edit menu",
    },
    CommandSpec {
        name: "load",
        arg_hint: Some("<file>"),
        description: "replace measures from file",
    },
];

#[derive(Debug, Clone)]
pub struct App {
    pub should_quit: bool,
    pub status: String,
    pub dirty: bool,
    pub selected_measure: usize,
    pub measures: Vec<MeasureRecord>,
    pub command_buffer: String,
    pub command_cursor: usize,
    pub command_input_mode: InputMode,
    pub show_help: bool,
    pub command_completion_index: usize,
    pub load_picker_open: bool,
    pub load_picker_query: String,
    pub load_picker_selection: usize,
    pub load_picker_candidates: Vec<String>,
    pub focused_panel: FocusedPanel,
    pub edit_menu_open: bool,
    pub edit_menu_selection: usize,
    pub edit_field_input_mode: bool,
    pub edit_field_input: String,
    pub edit_field_cursor: usize,
    pub edit_field_input_mode_mode: InputMode,
    history: Vec<MeasureHistoryEntry>,
    pub next_history_id: usize,
}

impl App {
    const MAX_COMMAND_LENGTH: usize = 128;
    const MAX_HISTORY_ENTRIES: usize = 32;

    pub fn new() -> Self {
        Self {
            should_quit: false,
            status: "No measures loaded. Press / then /load to start.".to_string(),
            dirty: false,
            selected_measure: 0,
            measures: Vec::new(),
            command_buffer: String::new(),
            command_cursor: 0,
            command_input_mode: InputMode::Insert,
            show_help: false,
            command_completion_index: 0,
            load_picker_open: false,
            load_picker_query: String::new(),
            load_picker_selection: 0,
            load_picker_candidates: Vec::new(),
            focused_panel: FocusedPanel::Measures,
            edit_menu_open: false,
            edit_menu_selection: 0,
            edit_field_input_mode: false,
            edit_field_input: String::new(),
            edit_field_cursor: 0,
            edit_field_input_mode_mode: InputMode::Insert,
            history: Vec::new(),
            next_history_id: 1,
        }
    }

    pub fn is_edit_mode(&self) -> bool {
        self.edit_menu_open
    }

    pub fn is_edit_value_mode(&self) -> bool {
        self.edit_menu_open && self.edit_field_input_mode
    }

    pub fn is_command_insert_mode(&self) -> bool {
        matches!(self.command_input_mode, InputMode::Insert)
    }

    pub fn is_command_normal_mode(&self) -> bool {
        matches!(self.command_input_mode, InputMode::Normal)
    }

    pub fn is_edit_field_insert_mode(&self) -> bool {
        matches!(self.edit_field_input_mode_mode, InputMode::Insert)
    }

    pub fn is_edit_field_normal_mode(&self) -> bool {
        matches!(self.edit_field_input_mode_mode, InputMode::Normal)
    }

    pub fn set_command_insert_mode(&mut self) {
        self.command_input_mode = InputMode::Insert;
    }

    pub fn set_command_normal_mode(&mut self) {
        self.command_input_mode = InputMode::Normal;
    }

    pub fn set_edit_field_insert_mode(&mut self) {
        self.edit_field_input_mode_mode = InputMode::Insert;
    }

    pub fn set_edit_field_normal_mode(&mut self) {
        self.edit_field_input_mode_mode = InputMode::Normal;
    }

    pub fn focused_panel(&self) -> FocusedPanel {
        self.focused_panel
    }

    pub fn is_measures_panel_focused(&self) -> bool {
        matches!(self.focused_panel, FocusedPanel::Measures)
    }

    pub fn focus_measures_panel(&mut self) {
        self.focused_panel = FocusedPanel::Measures;
    }

    pub fn focus_details_panel(&mut self) {
        self.focused_panel = FocusedPanel::Details;
    }

    pub fn focus_next_panel(&mut self) {
        self.focused_panel = self.focused_panel.next();
    }

    pub fn focus_previous_panel(&mut self) {
        self.focused_panel = self.focused_panel.prev();
    }

    fn next_history_id(&mut self) -> usize {
        let id = self.next_history_id;
        self.next_history_id += 1;
        id
    }

    fn record_history_entry(&mut self, action: impl Into<String>) {
        let mut selected_measure = self.selected_measure;
        if self.measures.is_empty() {
            selected_measure = 0;
        } else if selected_measure >= self.measures.len() {
            selected_measure = self.measures.len() - 1;
        }

        let entry = MeasureHistoryEntry {
            id: self.next_history_id(),
            action: action.into(),
            selected_measure,
            measures: self.measures.clone(),
        };
        self.history.push(entry);
        if self.history.len() > Self::MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.dirty = false;
    }

    pub fn history_status_lines(&self, max: usize) -> Vec<String> {
        self.history
            .iter()
            .take(max)
            .enumerate()
            .map(|(index, entry)| {
                format!(
                    "#{:<3} checkpoint {} | action: {} | selected #{} | {} measures",
                    index + 1,
                    entry.id,
                    entry.action,
                    entry.selected_measure + 1,
                    entry.measures.len()
                )
            })
            .collect()
    }

    pub fn restore_history_entry(&mut self, index: usize) -> Result<usize, String> {
        if self.history.is_empty() {
            return Err("No checkpoints available. Save first.".to_string());
        }
        let entry_index = if index == 0 { None } else { index.checked_sub(1) };
        let entry = self
            .history
            .get(entry_index.ok_or_else(|| "Checkpoint index must be >= 1.".to_string())?)
            .cloned()
            .ok_or_else(|| format!("Checkpoint #{index} does not exist."))?;

        let source_id = entry.id;
        self.measures = entry.measures.clone();
        self.selected_measure = if self.measures.is_empty() {
            0
        } else {
            entry.selected_measure.min(self.measures.len() - 1)
        };
        self.record_history_entry(format!("restore checkpoint #{source_id}"));
        Ok(self
            .history
            .last()
            .map(|entry| entry.id)
            .unwrap_or_default())
    }

    pub fn revert_previous_checkpoint(&mut self) -> Result<usize, String> {
        let previous_index = self
            .history
            .len()
            .checked_sub(2)
            .ok_or_else(|| "No earlier checkpoint to revert to. Save at least twice first.".to_string())?;

        let entry = self
            .history
            .get(previous_index)
            .cloned()
            .ok_or_else(|| "Unable to locate previous checkpoint.".to_string())?;

        self.measures = entry.measures.clone();
        self.selected_measure = if self.measures.is_empty() {
            0
        } else {
            entry.selected_measure.min(self.measures.len() - 1)
        };
        self.record_history_entry(format!("revert to checkpoint #{}", entry.id));
        Ok(self
            .history
            .last()
            .map(|entry| entry.id)
            .ok_or_else(|| "Unable to create revert checkpoint.".to_string())?)
    }

    pub fn edit_menu_fields(&self) -> &'static [EditableField] {
        &EDITABLE_MENU_FIELDS
    }

    pub fn edit_menu_field_value(&self, field: EditableField) -> String {
        let record = self.selected_measure_record();
        if record.is_none() {
            return "—".to_string();
        }

        let record = record.unwrap();
        match field {
            EditableField::Name => record.name.clone(),
            EditableField::Description => {
                record.description.clone().unwrap_or_else(|| "—".to_string())
            }
            EditableField::Reference => {
                record.reference.clone().unwrap_or_else(|| "—".to_string())
            }
            EditableField::Waves => record.waves.clone().unwrap_or_else(|| "—".to_string()),
            EditableField::Keywords => record.keywords.clone().unwrap_or_else(|| "—".to_string()),
            EditableField::Standardised => record.standardised.unwrap_or(false).to_string(),
            EditableField::StandardisedDate => record
                .standardised_date
                .clone()
                .unwrap_or_else(|| "—".to_string()),
            EditableField::Items => {
                if record.items.is_empty() {
                    "—".to_string()
                } else {
                    record.items.join(", ")
                }
            }
        }
    }

    pub fn selected_edit_field(&self) -> Option<EditableField> {
        self.edit_menu_fields()
            .get(self.edit_menu_selection)
            .copied()
    }

    pub fn open_edit_menu(&mut self) {
        if self.measures.is_empty() {
            self.status = "No measure selected. Load a dataset first.".to_string();
            return;
        }

        self.edit_menu_open = true;
        self.focused_panel = FocusedPanel::Details;
        self.edit_menu_selection = 0;
        self.edit_field_input_mode = false;
        self.edit_field_input.clear();
        self.edit_field_cursor = 0;
        self.status = "Edit menu: j/k to move, Enter to edit, Space to toggle, Esc to close.".to_string();
    }

    pub fn close_edit_menu(&mut self) {
        self.edit_menu_open = false;
        self.edit_menu_selection = 0;
        self.edit_field_input_mode = false;
        self.edit_field_input.clear();
        self.edit_field_cursor = 0;
        self.edit_field_input_mode_mode = InputMode::Insert;
    }

    pub fn next_edit_field(&mut self) {
        let len = self.edit_menu_fields().len();
        if len == 0 {
            return;
        }
        self.edit_menu_selection = (self.edit_menu_selection + 1) % len;
    }

    pub fn prev_edit_field(&mut self) {
        let len = self.edit_menu_fields().len();
        if len == 0 {
            return;
        }
        self.edit_menu_selection = if self.edit_menu_selection == 0 {
            len - 1
        } else {
            self.edit_menu_selection - 1
        };
    }

    pub fn begin_edit_field_input(&mut self) {
        let Some(field) = self.selected_edit_field() else {
            return;
        };

        if field.is_toggleable() {
            if let Some(selected) = self.measures.get_mut(self.selected_measure) {
                selected.standardised = Some(!selected.standardised.unwrap_or(false));
            }
            self.dirty = true;
            self.status = format!(
                "Toggled {} for {} to {}",
                field.label(),
                self.selected_measure_name().unwrap_or("record"),
                self.edit_menu_field_value(field)
            );
            self.command_completion_index = 0;
            return;
        }

        let value = self.edit_menu_field_value(field);
        self.edit_field_input = if value == "—" { String::new() } else { value };
        self.edit_field_input_mode = true;
        self.edit_field_cursor = self.edit_field_input.len();
        self.edit_field_input_mode_mode = InputMode::Insert;
        self.status = format!("Editing {}. Type new value and Enter to save.", field.label());
    }

    pub fn close_edit_field_input(&mut self) {
        self.edit_field_input_mode = false;
        self.edit_field_input.clear();
        self.edit_field_cursor = 0;
        self.edit_field_input_mode_mode = InputMode::Insert;
        self.status = "Edit value entry cancelled.".to_string();
    }

    pub fn edit_field_input_push_char(&mut self, ch: char) {
        if self.edit_field_input.len() < 512 {
            self.edit_field_input.insert(self.edit_field_cursor, ch);
            self.edit_field_cursor += ch.len_utf8();
        }
    }

    pub fn edit_field_input_backspace(&mut self) {
        if self.edit_field_cursor == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.edit_field_input, self.edit_field_cursor);
        self.edit_field_input.remove(prev);
        self.edit_field_cursor = prev;
    }

    pub fn edit_field_input_delete(&mut self) {
        if self.edit_field_cursor >= self.edit_field_input.len() {
            return;
        }
        let next = next_char_boundary(&self.edit_field_input, self.edit_field_cursor);
        if next > self.edit_field_cursor {
            self.edit_field_input
                .drain(self.edit_field_cursor..next);
        }
    }

    pub fn edit_field_move_cursor_left(&mut self) {
        self.edit_field_cursor = prev_char_boundary(&self.edit_field_input, self.edit_field_cursor);
    }

    pub fn edit_field_move_cursor_right(&mut self) {
        self.edit_field_cursor = next_char_boundary(&self.edit_field_input, self.edit_field_cursor);
    }

    pub fn commit_edit_field_input(&mut self) {
        let Some(field) = self.selected_edit_field() else {
            return;
        };

        let value = self.edit_field_input.trim().to_string();
        if value.is_empty() {
            self.edit_field_input_mode = false;
            self.edit_field_input.clear();
            self.edit_field_cursor = 0;
            self.edit_field_input_mode_mode = InputMode::Insert;
            self.status = format!("No value entered for {}. Kept current value.", field.label());
            return;
        }

        match self.apply_measure_field_edit(&format!("{} {}", field.key(), value)) {
            Ok(message) => {
                self.status = message;
            }
            Err(message) => {
                self.status = format!("Edit failed: {message}");
            }
        }
        self.edit_field_input_mode = false;
        self.edit_field_input.clear();
        self.edit_field_cursor = 0;
        self.edit_field_input_mode_mode = InputMode::Insert;
    }

    pub fn validate_measure(&self, path: &str) -> MeasureValidation {
        let value = path.trim();

        let has_text = !value.is_empty();
        let valid_segments = value
            .split('.')
            .filter(|segment| !segment.is_empty())
            .all(|segment| is_snake_case_segment(segment));
        let has_reference = has_text && valid_segments;

        MeasureValidation {
            description: has_text,
            reference: has_reference,
            waves: has_text && has_reference,
            keywords: has_text && has_reference,
            standardised: has_text && has_reference,
        }
    }

    pub fn selected_measure_record(&self) -> Option<&MeasureRecord> {
        self.measures.get(self.selected_measure)
    }

    pub fn selected_measure_name(&self) -> Option<&str> {
        self.measures
            .get(self.selected_measure)
            .map(|measure| measure.name.as_str())
    }

    pub fn next_measure(&mut self) {
        if self.measures.is_empty() {
            return;
        }
        self.selected_measure = (self.selected_measure + 1).min(self.measures.len() - 1);
    }

    pub fn prev_measure(&mut self) {
        if self.measures.is_empty() {
            return;
        }
        self.selected_measure = self.selected_measure.saturating_sub(1);
    }

    pub fn add_measure(&mut self, path: impl Into<String>) {
        let record = MeasureRecord::from_name(path.into());
        let status_suffix = if self.validate_measure(&record.name).is_valid() {
            "added"
        } else {
            "added (validation warning)"
        };
        let name = record.name.clone();
        self.measures.push(record);
        self.selected_measure = self.measures.len() - 1;
        self.dirty = true;
        self.status = format!("{status_suffix}: {name}");
        self.command_completion_index = 0;
    }

    pub fn remove_selected(&mut self) {
        if self.measures.is_empty() {
            self.status = "No measure selected.".to_string();
            return;
        }
        self.measures.remove(self.selected_measure);
        if self.selected_measure > 0 {
            self.selected_measure -= 1;
        } else if self.measures.is_empty() {
            self.selected_measure = 0;
        }
        self.dirty = true;
        self.status = "Removed measure. Save needed.".to_string();
        self.command_completion_index = 0;
    }

    pub fn save_stub(&mut self) {
        if self.measures.is_empty() {
            self.status = "No measures to save.".to_string();
            return;
        }
        self.record_history_entry("save");
        let newest = self
            .history
            .last()
            .map(|entry| entry.id)
            .unwrap_or_default();
        self.status = format!("Saved checkpoint #{newest}.");
        self.command_completion_index = 0;
    }


    pub fn begin_command(&mut self) {
        if !self.command_buffer.starts_with('/') {
            self.command_buffer = String::from("/");
        }
        self.command_cursor = self.command_buffer.len();
        self.command_input_mode = InputMode::Insert;
        self.command_completion_index = 0;
    }

    pub fn is_load_picker_open(&self) -> bool {
        self.load_picker_open
    }

    pub fn clear_command(&mut self) {
        self.command_buffer.clear();
        self.command_cursor = 0;
        self.command_completion_index = 0;
        self.command_input_mode = InputMode::Insert;
    }

    pub fn push_command_char(&mut self, ch: char) {
        if self.command_buffer.len() < Self::MAX_COMMAND_LENGTH {
            self.command_buffer.insert(self.command_cursor, ch);
            self.command_cursor += ch.len_utf8();
            self.command_completion_index = 0;
        }
    }

    pub fn command_backspace(&mut self) {
        if self.command_cursor == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.command_buffer, self.command_cursor);
        self.command_buffer.remove(prev);
        self.command_cursor = prev;
        if self.command_buffer.is_empty() {
            self.command_buffer.push('/');
            self.command_cursor = self.command_buffer.len();
        }
        self.command_completion_index = 0;
    }

    pub fn command_move_cursor_left(&mut self) {
        self.command_cursor = prev_char_boundary(&self.command_buffer, self.command_cursor);
    }

    pub fn command_move_cursor_right(&mut self) {
        self.command_cursor = next_char_boundary(&self.command_buffer, self.command_cursor);
    }

    pub fn command_input_delete(&mut self) {
        if self.command_cursor >= self.command_buffer.len() {
            return;
        }
        let next = next_char_boundary(&self.command_buffer, self.command_cursor);
        if next > self.command_cursor {
            self.command_buffer.drain(self.command_cursor..next);
        }
    }

    pub fn is_command_mode(&self) -> bool {
        self.command_buffer.starts_with('/')
    }

    pub fn command_suggestions(&self) -> Vec<String> {
        let token = self
            .command_buffer
            .splitn(2, ' ')
            .next()
            .unwrap_or("/")
            .trim_start_matches('/')
            .to_lowercase();

        let mut suggestions = COMMAND_SPECS
            .iter()
            .filter(|spec| token.is_empty() || spec.name.starts_with(&token))
            .map(|spec| {
                if let Some(arg_hint) = spec.arg_hint {
                    format!("/{:<6} {}", spec.name, arg_hint)
                } else {
                    format!("/{}", spec.name)
                }
            })
            .collect::<Vec<_>>();

        if suggestions.is_empty() {
            suggestions.push("no matches".to_string());
        }
        suggestions
    }

    pub fn auto_complete_command(&mut self) {
        let token = self
            .command_buffer
            .splitn(2, ' ')
            .next()
            .unwrap_or("/")
            .trim_start_matches('/')
            .to_lowercase();

        if token.is_empty() {
            self.status = "type a command name after /".to_string();
            return;
        }

        let matches: Vec<&CommandSpec> = COMMAND_SPECS
            .iter()
            .filter(|spec| spec.name.starts_with(&token))
            .collect();

        if matches.is_empty() {
            self.status = "No matching commands".to_string();
            return;
        }

        let index = self.command_completion_index % matches.len();
        let spec = matches[index];
        let rest = self
            .command_buffer
            .split_once(' ')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_default();
        self.command_buffer = format!("/{}", spec.name);
        if !rest.is_empty() {
            self.command_buffer.push(' ');
            self.command_buffer.push_str(&rest);
        } else if spec.arg_hint.is_some() {
            self.command_buffer.push(' ');
        }
        self.command_cursor = self.command_buffer.len();
        self.command_input_mode = InputMode::Insert;
        self.command_completion_index = (self.command_completion_index + 1) % matches.len();
        self.status = format!("command autocomplete: {}", spec.description);
    }

    pub fn open_load_picker(&mut self) {
        self.load_picker_open = true;
        self.load_picker_query.clear();
        self.load_picker_selection = 0;

        match discover_measure_files_for_picker() {
            Ok(paths) if paths.is_empty() => {
                self.load_picker_open = false;
                self.status = "No candidate measure files found. Use /load <path> to specify manually.".to_string();
            }
            Ok(paths) => {
                self.load_picker_candidates = sort_by_recency(paths);
                let count = self.load_picker_candidates.len();
                self.load_picker_selection = 0;
                self.status = format!(
                    "Load picker open ({count} candidates). Most recent file is preselected; Enter to load."
                );
            }
            Err(err) => {
                self.load_picker_open = false;
                self.status = format!("Unable to open file picker: {err}");
            }
        }
    }

    pub fn close_load_picker(&mut self) {
        self.load_picker_open = false;
        self.load_picker_query.clear();
        self.load_picker_selection = 0;
    }

    pub fn load_picker_push_char(&mut self, ch: char) {
        self.load_picker_query.push(ch);
        self.load_picker_selection = 0;
    }

    pub fn load_picker_backspace(&mut self) {
        self.load_picker_query.pop();
        self.load_picker_selection = 0;
    }

    pub fn next_load_picker_item(&mut self) {
        let len = self.load_picker_candidates().len();
        if len == 0 {
            return;
        }
        self.load_picker_selection = if self.load_picker_selection + 1 >= len {
            0
        } else {
            self.load_picker_selection + 1
        };
    }

    pub fn prev_load_picker_item(&mut self) {
        let len = self.load_picker_candidates().len();
        if len == 0 {
            return;
        }
        self.load_picker_selection = if self.load_picker_selection == 0 {
            len - 1
        } else {
            self.load_picker_selection - 1
        };
    }

    pub fn confirm_load_picker_selection(&mut self) -> bool {
        let candidates = self.load_picker_candidates();
        if candidates.is_empty() {
            self.status = "No matching files. Keep typing, or use /load <path>.".to_string();
            return false;
        }

        let index = self.load_picker_selection.min(candidates.len() - 1);
        let path = candidates[index].clone();
        match self.load_measure_file(&path) {
            Ok((count, loaded_path)) => {
                self.status = format!(
                    "Loaded {count} measures from {loaded_path}. Next: j/k to browse, /find <query> to jump."
                );
                self.close_load_picker();
                true
            }
            Err(err) => {
                self.status = format!("Unable to load: {err}");
                false
            }
        }
    }

    pub fn load_picker_candidates(&self) -> Vec<String> {
        let query = self.load_picker_query.trim();
        let mut matches: Vec<(i32, u64, String)> = self
            .load_picker_candidates
            .iter()
            .filter_map(|path| match fuzzy_score_measure_path(query, path) {
                Some(score) => Some((score, path_modified_unix(path), path.clone())),
                None => None,
            })
            .collect();

        matches.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.2.cmp(&b.2))
        });

        matches
            .into_iter()
            .map(|(_, _, path)| path)
            .collect::<Vec<_>>()
    }

    pub fn execute_command(&mut self) {
        let command_line = self.command_buffer.trim().to_string();
        let trimmed = command_line.trim_start_matches('/');
        let mut chunks = trimmed.splitn(2, ' ');
        let cmd = chunks.next().unwrap_or("").trim().to_lowercase();
        let arg = chunks.next().unwrap_or("").trim().to_string();
        self.command_buffer.clear();
        self.command_cursor = 0;
        self.command_input_mode = InputMode::Insert;

        if cmd.is_empty() {
            self.status = "Type a command after /".to_string();
            return;
        }
        let command = cmd.trim_matches('/').trim();

        match command {
            "help" => {
                self.show_help = true;
                self.status = "Opened help overlay".to_string();
            }
            "save" => {
                self.save_stub();
            }
            "quit" | "q" => self.should_quit = true,
            "clear" => {
                self.status = "Status cleared".to_string();
            }
            "reload" => {
                self.status = "Reload not yet implemented".to_string();
            }
            "add" => {
                if arg.is_empty() {
                    self.add_measure("new.measure.path");
                } else {
                    self.add_measure(&arg);
                }
            }
            "find" => {
                if arg.is_empty() {
                    self.status = "Usage: /find <query>".to_string();
                } else if self.find_measure(&arg) {
                    self.status = format!("Found: {arg}");
                } else {
                    self.status = format!("No measure matched: {arg}");
                }
            }
            "edit" => {
                if arg.is_empty() {
                    self.status = format!(
                        "Usage: /edit <field> <value>. Fields: {}",
                        EDITABLE_FIELDS.join(", ")
                    );
                } else {
                    match self.apply_measure_field_edit(&arg) {
                        Ok(message) => {
                            self.status = message;
                        }
                        Err(message) => {
                            self.status = format!("Edit failed: {message}");
                        }
                    }
                }
            }
            "history" => {
                if self.history.is_empty() {
                    self.status = "No checkpoint history yet. Use /save to create entries.".to_string();
                } else {
                    let entries = self
                        .history_status_lines(Self::MAX_HISTORY_ENTRIES)
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" | ");
                    self.status = format!("history: {entries}");
                }
            }
            "restore" => {
                if arg.is_empty() {
                    self.status = "Usage: /restore <index> (from /history list)".to_string();
                } else if let Ok(index) = arg.parse::<usize>() {
                    match self.restore_history_entry(index) {
                        Ok(new_id) => {
                            self.status = format!("Restored checkpoint {index} into new checkpoint #{new_id}.");
                        }
                        Err(message) => self.status = format!("Restore failed: {message}"),
                    }
                } else {
                    self.status = "Restore requires a numeric index (from /history).".to_string();
                }
            }
            "revert" | "undo" => match self.revert_previous_checkpoint() {
                Ok(new_id) => {
                    self.status = format!("Reverted to previous checkpoint as checkpoint #{new_id}.");
                }
                Err(message) => self.status = format!("Revert failed: {message}"),
            },
            "e" => {
                self.open_edit_menu();
            }
            "load" => {
                if arg.is_empty() {
                    self.open_load_picker();
                } else {
                    match self.load_measure_file(&arg) {
                        Ok((count, path)) => {
                            self.status = format!(
                                "Loaded {count} measures from {path}. Next: j/k to browse, /find <query> to jump."
                            );
                        }
                        Err(err) => self.status = format!("Unable to load: {err}"),
                    }
                }
            },
            _ => self.status = format!("Unknown command: /{command}"),
        }
        self.command_buffer.clear();
        self.command_completion_index = 0;
    }

    pub fn apply_measure_field_edit(&mut self, arg: &str) -> std::result::Result<String, String> {
        let mut parts = arg.splitn(2, ' ');
        let field = parts.next().unwrap_or("").trim().to_lowercase();
        let value = parts.next().unwrap_or("").trim();

        if field.is_empty() {
            return Err("Missing field name. Example: /edit name clinical.primary_outcome".to_string());
        }
        if value.is_empty() {
            return Err(format!(
                "Missing value for '{field}'. Try: /edit <field> <value>. Fields: {}",
                EDITABLE_FIELDS.join(", ")
            ));
        }

        let selected = self
            .measures
            .get_mut(self.selected_measure)
            .ok_or_else(|| "No measure selected. Load measures first.".to_string())?;

        match field.as_str() {
            "name" | "path" => {
                let cleaned = sanitize_measure(value.to_string())
                    .ok_or_else(|| "name cannot be empty".to_string())?;
                selected.name = cleaned;
            }
            "description" => {
                selected.description = Some(value.to_string());
            }
            "reference" => {
                selected.reference = Some(value.to_string());
            }
            "waves" => {
                selected.waves = Some(value.to_string());
            }
            "keywords" => {
                selected.keywords = Some(value.to_string());
            }
            "standardised" => {
                selected.standardised = Some(match value.to_lowercase().as_str() {
                    "true" | "t" | "1" | "yes" | "y" => true,
                    "false" | "f" | "0" | "no" | "n" => false,
                    _ => {
                        return Err("standardised must be true or false".to_string());
                    }
                });
            }
            "standardised_date" => {
                selected.standardised_date = Some(value.to_string());
            }
            "items" => {
                let items = value
                    .split([',', ';'])
                    .map(|item| item.trim())
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                selected.items = items;
            }
            _ => {
                return Err(format!(
                    "Unknown field '{field}'. Fields: {}",
                    EDITABLE_FIELDS.join(", ")
                ));
            }
        }

        self.dirty = true;
        self.command_completion_index = 0;
        Ok(format!(
            "Updated {} for {}",
            field,
            selected.name
        ))
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn find_measure(&mut self, query: &str) -> bool {
        if self.measures.is_empty() {
            self.status = "No measures loaded".to_string();
            return false;
        }

        let query = query.to_lowercase();
        let mut start = self.selected_measure.saturating_add(1);
        if start >= self.measures.len() {
            start = 0;
        }

        for offset in 0..self.measures.len() {
            let idx = (start + offset) % self.measures.len();
            if self.measures[idx].name.to_lowercase().contains(&query) {
                self.selected_measure = idx;
                return true;
            }
        }
        false
    }

    pub fn load_measure_file(&mut self, path: &str) -> Result<(usize, String)> {
        let path = path.trim();
        if path.is_empty() {
            return Err(anyhow!("Usage: /load <path-to-measures-file>"));
        }

        let path = unquote_path(path);
        if path.is_empty() {
            return Err(anyhow!("Usage: /load <path-to-measures-file>"));
        }

        let resolved_path = expand_path(&path)?;
        let content = fs::read_to_string(&resolved_path)
            .with_context(|| format!("Unable to read file: {}", resolved_path.display()))?;
        let parsed = parse_measure_payload(&content);
        if parsed.is_empty() {
            return Err(anyhow!(
                "No valid measure entries in {}",
                resolved_path.display()
            ));
        }

        let mut parsed_unique = Vec::new();
        let mut seen = HashSet::new();
        for value in parsed {
            let mut value = value;
            if let Some(cleaned) = sanitize_measure(value.name) {
                value.name = cleaned;
            } else {
                continue;
            }

            if seen.insert(value.name.clone()) {
                parsed_unique.push(value);
            }
        }

        if parsed_unique.is_empty() {
            return Err(anyhow!(
                "All measure rows were duplicates in {}",
                resolved_path.display()
            ));
        }

        self.measures = parsed_unique;
        self.selected_measure = 0;
        let loaded_path = resolved_path.display().to_string();
        self.history.clear();
        self.next_history_id = 1;
        self.record_history_entry(format!("load {loaded_path}"));
        self.dirty = false;
        self.command_completion_index = 0;
        Ok((self.measures.len(), loaded_path))
    }
}

fn unquote_path(path: &str) -> String {
    let mut trimmed = path.trim();
    if (trimmed.starts_with('\"') && trimmed.ends_with('\"') && trimmed.len() > 1)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() > 1)
        || (trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() > 1)
    {
        trimmed = trimmed.trim_matches(&['\"', '\'', '`'][..]);
    }
    trimmed.to_string()
}

fn prev_char_boundary(text: &str, byte_index: usize) -> usize {
    if byte_index == 0 || text.is_empty() {
        return 0;
    }
    let idx = byte_index.min(text.len());
    idx - text[..idx].chars().next_back().map_or(0, |ch| ch.len_utf8())
}

fn next_char_boundary(text: &str, byte_index: usize) -> usize {
    if text.is_empty() || byte_index >= text.len() {
        return text.len();
    }
    let idx = byte_index.min(text.len());
    idx + text[idx..].chars().next().map_or(0, |ch| ch.len_utf8())
}

fn expand_path(raw_path: &str) -> Result<std::path::PathBuf> {
    if raw_path.starts_with("~/") {
        let home = std::env::var("HOME")
            .with_context(|| "Cannot resolve ~ (HOME not set)")?;
        return Ok(std::path::Path::new(&home).join(raw_path.trim_start_matches("~/")));
    }
    Ok(std::path::PathBuf::from(raw_path))
}

fn parse_measure_payload(raw: &str) -> Vec<MeasureRecord> {
    let text = raw.trim();

    if text.is_empty() {
        return Vec::new();
    }

    let normalised: Vec<MeasureRecord> = if looks_like_json_array(text)
        || (text.starts_with('{') && text.ends_with('}'))
    {
        serde_json::from_str::<Value>(raw)
            .map(|value| extract_measure_records(&value))
            .unwrap_or_else(|_| {
                parse_json_array(raw)
                    .map(|names| {
                        names
                            .into_iter()
                            .map(MeasureRecord::from_name)
                            .collect()
                    })
                    .unwrap_or_else(|_| {
                        parse_measure_lines(raw)
                            .into_iter()
                            .map(MeasureRecord::from_name)
                            .collect()
                    })
            })
    } else {
        parse_measure_lines(raw)
            .into_iter()
            .map(MeasureRecord::from_name)
            .collect()
    };

    normalised
        .into_iter()
        .filter(|value| !value.name.is_empty())
        .collect()
}

fn parse_json_array(raw: &str) -> std::result::Result<Vec<String>, serde_json::Error> {
    let parsed = serde_json::from_str::<Vec<String>>(raw)?;
    Ok(parsed)
}

fn extract_measure_names_from_json(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => {
            if let Some(measures) = map.get("measures") {
                return extract_measure_names_from_json(measures);
            }

            map.keys().map(ToString::to_string).collect()
        }
        Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn looks_like_json_array(raw: &str) -> bool {
    raw.starts_with('[') && raw.ends_with(']')
}

fn parse_measure_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn extract_measure_records(value: &Value) -> Vec<MeasureRecord> {
    let mut records = Vec::new();

    if let Some(root_map) = value.as_object() {
        if let Some(measures) = root_map.get("measures") {
            if measures.is_object() {
                records.extend(extract_measure_records(measures));
            } else if measures.is_array() {
                records.extend(
                    measures
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .filter_map(record_from_value),
                );
            }
        }

        for (name, entry) in root_map {
            if name == "measures" {
                continue;
            }

            if let Some(record) = record_from_value_with_key(name, entry) {
                records.push(record);
            }
        }

        if !records.is_empty() {
            return records;
        }
    }

    if value.is_array() {
        records.extend(
            value
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(record_from_value),
        );
        if !records.is_empty() {
            return records;
        }
    }

    extract_measure_names_from_json(value)
        .into_iter()
        .map(MeasureRecord::from_name)
        .collect()
}

fn record_from_value(value: &Value) -> Option<MeasureRecord> {
    match value {
        Value::String(name) => Some(MeasureRecord::from_name(name.clone())),
        Value::Object(map) => map.get("name").and_then(Value::as_str).map(|name| {
            MeasureRecord {
                name: name.to_string(),
                description: as_string_opt(map.get("description")),
                reference: as_string_opt(map.get("reference")),
                waves: as_string_opt(map.get("waves")),
                keywords: as_string_opt(map.get("keywords")),
                standardised: map.get("standardised").and_then(Value::as_bool),
                standardised_date: as_string_opt(map.get("standardised_date")),
                items: map
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flat_map(|items| items.iter())
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect(),
                ..MeasureRecord::default()
            }
        }),
        _ => None,
    }
}

fn record_from_value_with_key(name: &str, value: &Value) -> Option<MeasureRecord> {
    if value.is_string() || value.is_object() {
        if let Some(record) = record_from_value(value) {
            if !record.name.is_empty() {
                return Some(record);
            }
            return Some(MeasureRecord {
                name: name.to_string(),
                ..record
            });
        } else if value.is_string() {
            return Some(MeasureRecord::from_name(name.to_string()));
        }
    }

    if let Some(obj) = value.as_object() {
        let has_measure_fields = ["description", "reference", "waves", "keywords", "items", "standardised"]
            .iter()
            .any(|field| obj.contains_key(*field));
        if has_measure_fields {
            let record = MeasureRecord {
                name: name.to_string(),
                description: as_string_opt(obj.get("description")),
                reference: as_string_opt(obj.get("reference")),
                waves: as_string_opt(obj.get("waves")),
                keywords: as_string_opt(obj.get("keywords")),
                standardised: obj.get("standardised").and_then(Value::as_bool),
                standardised_date: as_string_opt(obj.get("standardised_date")),
                items: obj
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flat_map(|items| items.iter())
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect(),
            };
            return Some(record);
        }
    }

    None
}

fn as_string_opt(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .or_else(|| value.and_then(Value::as_bool).map(|value| value.to_string()))
}

fn sanitize_measure(value: String) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(&['"', '\'', ';', ','][..])
        .trim()
        .replace('\r', "");
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn is_snake_case_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && segment
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
}

fn discover_measure_files_for_picker() -> Result<Vec<String>> {
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.clone());
        if let Some(parent) = current.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let home_dir = PathBuf::from(home);
        roots.push(home_dir.join("GIT"));
        roots.push(home_dir.join("git"));
        roots.push(home_dir.join("templates"));
        roots.push(home_dir.join("projects"));
        roots.push(home_dir.join("Downloads"));
    }

    let mut root_dirs = HashSet::new();
    for root in roots {
        let canonical = root.to_string_lossy().to_string();
        root_dirs.insert(canonical);
    }

    let mut candidates = Vec::new();
    for root in root_dirs {
        let root = PathBuf::from(&root);
        if !root.is_dir() {
            continue;
        }
        collect_candidate_measure_files(&root, 0, 4, &mut candidates)?;
    }

    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn path_modified_unix(path: &str) -> u64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|timestamp| timestamp.as_secs())
        .unwrap_or(0)
}

fn sort_by_recency(mut paths: Vec<String>) -> Vec<String> {
    paths.sort_by(|a, b| {
        path_modified_unix(b)
            .cmp(&path_modified_unix(a))
            .then_with(|| a.cmp(b))
    });
    paths
}

fn collect_candidate_measure_files(
    path: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<String>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = fs::read_dir(path)
        .with_context(|| format!("Cannot scan directory: {}", path.display()))?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        if should_skip_directory_entry(&entry_path) {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_candidate_measure_files(&entry_path, depth + 1, max_depth, out)?;
        } else if is_measure_file_candidate(&entry_path) {
            out.push(entry_path.to_string_lossy().to_string());
        }
    }

    Ok(())
}

fn is_measure_file_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("json") || value.eq_ignore_ascii_case("ndjson"))
        .unwrap_or(false)
}

fn should_skip_directory_entry(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    let hidden = file_name.starts_with('.');
    let blacklisted = matches!(
        file_name,
        "target" | "node_modules" | ".git" | ".cache" | "dist" | "build" | "tmp" | "vendor"
    );

    hidden || blacklisted
}

fn fuzzy_score_measure_path(query: &str, raw_path: &str) -> Option<i32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(900);
    }

    let path = raw_path.to_lowercase();
    let path_file = Path::new(raw_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(raw_path)
        .to_lowercase();

    if let Some(index) = path.find(&query) {
        let mut score = 500 - (index as i32).min(250);
        if path_file.contains(&query) {
            score += 80;
        }
        return Some(score.max(0));
    }

    let mut cursor = 0usize;
    for ch in query.chars() {
        if path.len() <= cursor {
            return None;
        }
        let rest = &path[cursor..];
        let offset = rest.find(ch)?;
        cursor += offset + ch.len_utf8();
    }

    let mut score = (query.len() as i32) * 12 - (cursor as i32).min(200);
    if path_file.contains(&query) {
        score += 80;
    }
    if path.ends_with(".json") || path.ends_with(".ndjson") {
        score += 20;
    }
    if path.contains("boilerplate") {
        score += 30;
    }

    Some(score)
}
