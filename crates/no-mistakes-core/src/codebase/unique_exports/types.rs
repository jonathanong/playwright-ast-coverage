use crate::codebase::ts_symbols::{Export, FileSymbols};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UniqueExportsOptions {
    pub unique_across_types_and_values: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UniqueExportFinding {
    pub rule: String,
    pub file: String,
    pub line: u32,
    pub export_name: String,
    pub export_kind: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub(super) struct SourceFile {
    pub(super) path: PathBuf,
    pub(super) rel: String,
    pub(super) source: String,
    pub(super) symbols: FileSymbols,
    pub(super) disabled: bool,
    pub(super) is_nextjs_project: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum ExportBucket {
    Type,
    Value,
    Any,
}

impl ExportBucket {
    pub(super) fn from_export(export: &Export) -> Self {
        if export.is_type_only {
            Self::Type
        } else {
            Self::Value
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Value => "value",
            Self::Any => "export",
        }
    }

    pub(super) fn key(self, strict: bool) -> Self {
        if strict {
            Self::Any
        } else {
            self
        }
    }

    pub(super) fn message_label(self) -> &'static str {
        match self {
            Self::Type => "type export",
            Self::Value => "value export",
            Self::Any => "export",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ExportOccurrence {
    pub(super) name: String,
    pub(super) bucket: ExportBucket,
    pub(super) file: String,
    pub(super) line: u32,
    pub(super) kind: String,
    pub(super) origin: ExportOrigin,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct ExportOrigin {
    pub(super) file: String,
    pub(super) line: u32,
    pub(super) name: String,
    pub(super) bucket: ExportBucket,
}
