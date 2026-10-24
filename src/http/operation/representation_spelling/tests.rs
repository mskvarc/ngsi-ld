//! The spellings of Tables 6.3.7-1 and 6.3.12-1, written and read back.

use crate::{
    http::{
        QueryParameter,
        operation::{
            ArgumentError,
            ParameterError,
            RepresentationSpelling,
            RepresentationToken,
            SystemAttributes,
            TemporalRepresentation,
            received_parameters::ReceivedQuery,
            representation_spelling::{
                RepresentationFamily,
                received::{ReceivedRepresentation, read_representation},
            },
        },
        query_string::QueryString,
    },
    model::Representation,
};
use pretty_assertions::assert_eq;

/// The query string a request writes for `representation` in `spelling`, with or without `sysAttrs`.
fn written<R: RepresentationFamily>(representation: R, spelling: RepresentationSpelling, system_attributes: SystemAttributes) -> String {
    let mut query = QueryString::default();
    spelling.push_format(&mut query, representation);
    spelling.push_options(&mut query, representation, system_attributes);
    query.to_string()
}

/// Reads the representation of a received query string, of the family `R`.
fn read<R: RepresentationFamily>(query: &str) -> Result<ReceivedRepresentation<R>, ParameterError> {
    read_representation(&ReceivedQuery::read(query, &[&[QueryParameter::Format, QueryParameter::Options]]).unwrap())
}

#[test]
fn the_default_spelling_writes_format_and_leaves_normalized_out() {
    let spelling = RepresentationSpelling::default();
    assert_eq!(written(Representation::Normalized, spelling, SystemAttributes::Omit), "");
    assert_eq!(written(Representation::Concise, spelling, SystemAttributes::Omit), "format=concise");
    assert_eq!(
        written(Representation::Simplified, spelling, SystemAttributes::Include),
        "format=simplified&options=sysAttrs"
    );
    assert_eq!(
        written(TemporalRepresentation::Simplified, spelling, SystemAttributes::Omit),
        "format=temporalValues"
    );
    assert_eq!(
        written(TemporalRepresentation::Aggregated, spelling, SystemAttributes::Include),
        "format=aggregatedValues&options=sysAttrs"
    );
}

#[test]
fn every_spelling_of_tables_6_3_7_1_and_6_3_12_1_is_written() {
    let base = RepresentationSpelling::default();
    let omit = SystemAttributes::Omit;
    assert_eq!(written(Representation::Normalized, base.explicit_normalized(), omit), "format=normalized");
    assert_eq!(written(TemporalRepresentation::Normalized, base.explicit_normalized(), omit), "");
    assert_eq!(written(Representation::Simplified, base.key_values(), omit), "format=keyValues");
    assert_eq!(written(Representation::Concise, base.key_values(), omit), "format=concise");
    let options = base.in_options();
    assert_eq!(written(Representation::Normalized, options, SystemAttributes::Include), "options=sysAttrs");
    assert_eq!(written(Representation::Concise, options, omit), "options=concise");
    assert_eq!(
        written(Representation::Simplified, options.key_values(), SystemAttributes::Include),
        "options=keyValues,sysAttrs"
    );
    assert_eq!(written(Representation::Normalized, options.explicit_normalized(), omit), "options=normalized");
    assert_eq!(
        written(TemporalRepresentation::Simplified, options, SystemAttributes::Include),
        "options=temporalValues,sysAttrs"
    );
    assert_eq!(written(TemporalRepresentation::Aggregated, options, omit), "options=aggregatedValues");
}

#[test]
fn a_keyword_beside_the_format_never_changes_the_representation_read() {
    let aggregated = RepresentationSpelling::default().format_over_options(RepresentationToken::AggregatedValues);
    assert_eq!(
        written(TemporalRepresentation::Simplified, aggregated, SystemAttributes::Omit),
        "format=temporalValues&options=aggregatedValues"
    );
    assert!(matches!(
        aggregated.check(TemporalRepresentation::Normalized),
        Err(ArgumentError::OptionsKeywordWithoutFormat {
            keyword: RepresentationToken::AggregatedValues
        })
    ));
    assert!(aggregated.check(TemporalRepresentation::Simplified).is_ok());
    assert_eq!(
        written(Representation::Normalized, aggregated, SystemAttributes::Include),
        "format=normalized&options=aggregatedValues,sysAttrs"
    );
    let values = RepresentationSpelling::default().format_over_options(RepresentationToken::TemporalValues);
    assert_eq!(
        written(TemporalRepresentation::Concise, values, SystemAttributes::Omit),
        "format=concise&options=temporalValues"
    );
    let key_values = RepresentationSpelling::default().format_over_options(RepresentationToken::KeyValues);
    assert_eq!(
        written(Representation::Concise, key_values, SystemAttributes::Omit),
        "format=concise&options=keyValues"
    );
    let read = read::<TemporalRepresentation>("format=temporalValues&options=aggregatedValues").unwrap();
    assert_eq!(read.representation, TemporalRepresentation::Simplified);
    assert_eq!(read.spelling, aggregated);
}

#[test]
fn every_spelling_written_reads_back_as_written() {
    let base = RepresentationSpelling::default();
    let spellings = [
        base,
        base.explicit_normalized(),
        base.key_values(),
        base.in_options(),
        base.in_options().key_values(),
        base.format_over_options(RepresentationToken::Concise),
    ];
    for representation in [Representation::Normalized, Representation::Concise, Representation::Simplified] {
        for spelling in spellings {
            for system_attributes in [SystemAttributes::Omit, SystemAttributes::Include] {
                let read = read::<Representation>(&written(representation, spelling, system_attributes)).unwrap();
                assert_eq!(read.representation, representation);
                assert_eq!(read.system_attributes, system_attributes);
                let mut again = QueryString::default();
                read.spelling.push_format(&mut again, representation);
                read.spelling.push_options(&mut again, representation, system_attributes);
                assert_eq!(again.to_string(), written(representation, spelling, system_attributes));
            }
        }
    }
}

#[test]
fn a_token_of_another_table_or_a_second_keyword_is_refused() {
    assert!(matches!(
        read::<Representation>("format=compact"),
        Err(ParameterError::UnknownToken { parameter: QueryParameter::Format, token }) if token == "compact"
    ));
    assert!(matches!(
        read::<Representation>("format=temporalValues"),
        Err(ParameterError::UnknownToken {
            parameter: QueryParameter::Format,
            ..
        })
    ));
    assert!(matches!(
        read::<TemporalRepresentation>("options=keyValues"),
        Err(ParameterError::UnknownToken { parameter: QueryParameter::Options, token }) if token == "keyValues"
    ));
    assert!(matches!(
        read::<Representation>("options=noOverwrite"),
        Err(ParameterError::UnknownToken {
            parameter: QueryParameter::Options,
            ..
        })
    ));
    assert!(matches!(
        read::<TemporalRepresentation>("options=temporalValues,aggregatedValues"),
        Err(ParameterError::RepresentationTwice)
    ));
}

#[test]
fn the_tokens_are_spelled_as_the_tables_write_them() {
    assert_eq!("temporalValues".parse(), Ok(RepresentationToken::TemporalValues));
    assert_eq!(TemporalRepresentation::Aggregated.table_token(), Some(RepresentationToken::AggregatedValues));
    assert_eq!(TemporalRepresentation::Normalized.table_token(), None);
    assert_eq!(Representation::Simplified.table_token(), Some(RepresentationToken::Simplified));
}
