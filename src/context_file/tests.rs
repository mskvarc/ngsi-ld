use crate::{
    context_file::{self, ContextFileError},
    model::{Context, ContextEntry},
};
use iri_rs::IriRefBuf;
use jstrict::Value as Json;
use pretty_assertions::assert_eq;
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process,
};
use url::Url;

/// A fresh directory for one test, under the system's temporary directory.
fn directory(test: &str) -> PathBuf {
    let directory = env::temp_dir().join(format!("ngsi-ld-context-file-{}-{test}", process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir_all(directory.join("shared")).unwrap();
    directory
}

/// Writes `text` to `name` under `directory`.
fn write(directory: &Path, name: &str, text: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, text).unwrap();
    path
}

/// The context as written in a body.
fn json(context: &Context) -> Json {
    context.to_json()
}

#[test]
fn a_file_of_one_absolute_reference_stays_one_reference() {
    let directory = directory("absolute");
    let path = write(&directory, "fleet.jsonld", r#"{"@context": "https://example.org/fleet.jsonld"}"#);
    let context = context_file::load(&path).unwrap();
    assert_eq!(context, Context::from(IriRefBuf::new("https://example.org/fleet.jsonld").unwrap()));
    assert!(context.link_target().is_some());
}

#[test]
fn relative_references_are_loaded_against_the_file_that_holds_them_and_inlined_in_order() {
    let directory = directory("relative");
    write(
        &directory,
        "shared/units.jsonld",
        r#"{"@context": [{"unit": "https://example.org/unit"}, "https://example.org/core.jsonld"]}"#,
    );
    write(
        &directory,
        "shared/vehicle.jsonld",
        r#"{"@context": ["units.jsonld", {"speed": "https://example.org/speed"}]}"#,
    );
    let path = write(
        &directory,
        "fleet.jsonld",
        r#"{"@context": ["shared/vehicle.jsonld", {"Vehicle": "https://example.org/Vehicle"}, "https://example.org/fleet.jsonld"]}"#,
    );
    let context = context_file::load(&path).unwrap();
    assert_eq!(
        json(&context),
        r#"[{"unit":"https://example.org/unit"},"https://example.org/core.jsonld",{"speed":"https://example.org/speed"},{"Vehicle":"https://example.org/Vehicle"},"https://example.org/fleet.jsonld"]"#
            .parse::<Json>()
            .unwrap()
    );
    assert!(context.references().all(IriRefBuf::is_absolute));
}

#[test]
fn a_file_iri_reference_is_loaded_and_inlined() {
    let directory = directory("file-iri");
    let units = write(&directory, "shared/units.jsonld", r#"{"@context": {"unit": "https://example.org/unit"}}"#);
    let file_iri = Url::from_file_path(fs::canonicalize(&units).unwrap()).unwrap();
    let path = write(&directory, "fleet.jsonld", &format!(r#"{{"@context": "{file_iri}"}}"#));
    let context = context_file::load(&path).unwrap();
    assert_eq!(json(&context), r#"{"unit":"https://example.org/unit"}"#.parse::<Json>().unwrap());
    assert!(matches!(context, Context::Single(ContextEntry::Inline(_))));
}

#[test]
fn a_cycle_of_files_is_refused() {
    let directory = directory("cycle");
    write(&directory, "a.jsonld", r#"{"@context": ["b.jsonld"]}"#);
    let path = write(&directory, "b.jsonld", r#"{"@context": "a.jsonld"}"#);
    assert!(matches!(context_file::load(&path), Err(ContextFileError::Cycle { .. })));
}

#[test]
fn a_missing_file_a_document_without_context_and_a_null_context_are_refused() {
    let directory = directory("refusals");
    assert!(matches!(
        context_file::load(directory.join("absent.jsonld")),
        Err(ContextFileError::Read { .. })
    ));
    let bare = write(&directory, "bare.jsonld", r#"{"speed": "https://example.org/speed"}"#);
    assert!(matches!(context_file::load(&bare), Err(ContextFileError::Document { .. })));
    let null = write(&directory, "null.jsonld", r#"{"@context": null}"#);
    assert!(matches!(context_file::load(&null), Err(ContextFileError::Document { .. })));
    let referencing_absent = write(&directory, "dangling.jsonld", r#"{"@context": "absent.jsonld"}"#);
    assert!(matches!(context_file::load(&referencing_absent), Err(ContextFileError::Read { .. })));
}
