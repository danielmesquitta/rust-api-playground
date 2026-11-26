//! Todo request and response types.

use serde::{Deserialize, Deserializer, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};

/// Longest title allowed, in characters. The database enforces the same limit.
pub(crate) const TITLE_MAX_CHARS: usize = 200;

/// A stored todo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow, ToSchema)]
pub(crate) struct Todo {
    /// Unique identifier, assigned by the server.
    #[schema(example = 1)]
    pub(crate) id: i64,
    /// What needs doing.
    #[schema(example = "Buy milk")]
    pub(crate) title: String,
    /// Optional longer notes.
    #[schema(example = "Two litres, semi-skimmed")]
    pub(crate) description: Option<String>,
    /// Whether the todo is done.
    pub(crate) completed: bool,
    /// When the todo was created (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,
    /// When the todo was last changed (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) updated_at: OffsetDateTime,
}

/// A todo title: trimmed, non-empty, and at most [`TITLE_MAX_CHARS`] characters.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Title(String);

/// Why a string is not a valid [`Title`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InvalidTitleError {
    /// The title is empty or only whitespace.
    #[error("title must not be empty")]
    Empty,
    /// The title is longer than [`TITLE_MAX_CHARS`].
    #[error("title must be at most {TITLE_MAX_CHARS} characters")]
    TooLong,
}

impl Title {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Title {
    type Error = InvalidTitleError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(InvalidTitleError::Empty);
        }
        if trimmed.chars().count() > TITLE_MAX_CHARS {
            return Err(InvalidTitleError::TooLong);
        }
        // Reuse the allocation when there was nothing to trim.
        let title = if trimmed.len() == raw.len() {
            raw
        } else {
            trimmed.to_owned()
        };
        Ok(Self(title))
    }
}

/// The body of a create request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewTodo {
    /// What needs doing. Leading and trailing whitespace is trimmed.
    #[schema(value_type = String, min_length = 1, max_length = 200, example = "Buy milk")]
    pub(crate) title: Title,
    /// Optional longer notes.
    #[serde(default)]
    pub(crate) description: Option<String>,
}

/// The body of a partial update. Absent fields are left unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TodoPatch {
    /// A new title. Leading and trailing whitespace is trimmed.
    #[schema(value_type = Option<String>, min_length = 1, max_length = 200)]
    #[serde(default)]
    pub(crate) title: Option<Title>,
    /// A new description; `null` clears it, absent leaves it unchanged.
    #[schema(value_type = Option<String>)]
    #[serde(default)]
    pub(crate) description: Change<String>,
    /// Marks the todo done or not done.
    #[serde(default)]
    pub(crate) completed: Option<bool>,
}

/// Query parameters for listing todos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListParams {
    /// Only return todos with this completion state.
    pub(crate) completed: Option<bool>,
}

/// What a patch does to a nullable field: an absent field keeps its value (via
/// `#[serde(default)]`), `null` clears it, and any other value replaces it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Change<T> {
    /// Leave the field as it is.
    #[default]
    Keep,
    /// Set the field to `NULL`.
    Clear,
    /// Replace the field's value.
    Set(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Change<T> {
    // Only called when the field is present, so a missing field stays `Keep`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::plain("Buy milk", Ok("Buy milk"))]
    #[case::trims_whitespace("  Buy milk \n", Ok("Buy milk"))]
    #[case::empty("", Err(InvalidTitleError::Empty))]
    #[case::only_whitespace(" \t\n ", Err(InvalidTitleError::Empty))]
    #[case::at_limit(&"a".repeat(200), Ok(&*"a".repeat(200)))]
    #[case::over_limit(&"a".repeat(201), Err(InvalidTitleError::TooLong))]
    #[case::counts_chars_not_bytes(&"é".repeat(200), Ok(&*"é".repeat(200)))]
    #[case::multibyte_over_limit(&"é".repeat(201), Err(InvalidTitleError::TooLong))]
    fn title_validation(#[case] raw: &str, #[case] expected: Result<&str, InvalidTitleError>) {
        let title = Title::try_from(raw.to_owned());

        assert_eq!(
            title.as_ref().map(Title::as_str),
            expected.as_ref().copied()
        );
    }

    #[rstest]
    #[case::title_only(r#"{"title":"Buy milk"}"#, Some(("Buy milk", None)))]
    #[case::with_description(r#"{"title":"Buy milk","description":"2L"}"#, Some(("Buy milk", Some("2L"))))]
    #[case::null_description(r#"{"title":"Buy milk","description":null}"#, Some(("Buy milk", None)))]
    #[case::trims_title(r#"{"title":"  Buy milk  "}"#, Some(("Buy milk", None)))]
    #[case::missing_title(r#"{"description":"2L"}"#, None)]
    #[case::empty_title(r#"{"title":""}"#, None)]
    #[case::unknown_field(r#"{"title":"Buy milk","done":true}"#, None)]
    #[case::wrong_type(r#"{"title":42}"#, None)]
    fn new_todo_deserialization(
        #[case] json: &str,
        #[case] expected: Option<(&str, Option<&str>)>,
    ) {
        let parsed = serde_json::from_str::<NewTodo>(json);

        match expected {
            Some((title, description)) => {
                let new = parsed.expect("body should be accepted");
                assert_eq!(new.title.as_str(), title);
                assert_eq!(new.description.as_deref(), description);
            }
            None => assert!(parsed.is_err(), "expected rejection, got {parsed:?}"),
        }
    }

    #[rstest]
    #[case::empty_leaves_everything(r"{}", Some(TodoPatch::default()))]
    #[case::null_clears_description(
        r#"{"description":null}"#,
        Some(TodoPatch { description: Change::Clear, ..TodoPatch::default() }),
    )]
    #[case::sets_description(
        r#"{"description":"2L"}"#,
        Some(TodoPatch { description: Change::Set("2L".to_owned()), ..TodoPatch::default() }),
    )]
    #[case::sets_completed(
        r#"{"completed":true}"#,
        Some(TodoPatch { completed: Some(true), ..TodoPatch::default() }),
    )]
    #[case::sets_title(
        r#"{"title":" New "}"#,
        Some(TodoPatch { title: Some(Title("New".to_owned())), ..TodoPatch::default() }),
    )]
    #[case::null_title_is_unchanged(r#"{"title":null}"#, Some(TodoPatch::default()))]
    #[case::empty_title(r#"{"title":""}"#, None)]
    #[case::unknown_field(r#"{"done":true}"#, None)]
    #[case::completed_wrong_type(r#"{"completed":"yes"}"#, None)]
    fn patch_deserialization(#[case] json: &str, #[case] expected: Option<TodoPatch>) {
        let parsed = serde_json::from_str::<TodoPatch>(json).ok();

        assert_eq!(parsed, expected);
    }
}
