//! Text input field handling.

/// State for a text input field.
#[derive(Clone, Debug, Default)]
pub struct TextInput {
    /// The current text content.
    pub content: String,
    /// Cursor position (character index).
    pub cursor: usize,
}

impl TextInput {
    /// Creates a new empty text input.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a character at the cursor position.
    pub fn insert(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Deletes the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.content.remove(self.cursor);
        }
    }

    /// Deletes the character at the cursor position (delete).
    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    /// Moves the cursor left.
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor right.
    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor += 1;
        }
    }

    /// Moves the cursor to the beginning.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end.
    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Takes the content and resets the input.
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.content)
    }

    /// Returns the current content as a string slice.
    pub fn as_str(&self) -> &str {
        &self.content
    }

    /// Returns whether the input is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}
