//! The query string of a request: typed parameters printed and percent-encoded.
//!
//! Which characters stay literal depends on the parameter, because a separator a broker must see is syntax, while
//! the same character inside a value is data:
//!
//! - a scalar value and the `q` and `csf` queries are opaque: every character but the unreserved ones (RFC 3986
//!   section 2.3) is escaped, since the syntax characters of the query language (clause 4.9) all sit inside one
//!   value that the broker decodes before parsing;
//! - a comma-separated list (`id`, `drop`, `keep`, the "comma separated list" parameters of clause 6) writes its
//!   separator as a literal `,` and escapes every item, a comma inside an item included, so a broker that splits
//!   the raw value before decoding it still sees one item per element;
//! - an entity type selection (`type`, clause 4.17) keeps its grammar characters `,`, `;`, `(` and `)` literal, all
//!   sub-delimiters RFC 3986 section 3.4 allows in a query, and so does the `*` that selects every type (Table
//!   6.4.3.2-1). Its `|` is not a URI character (RFC 3986 section 2), so it is escaped as `%7C`, which a broker
//!   decodes before reading the selection;
//! - a projection (`pick`, `omit`, clause 4.21) keeps its separator `,` literal, as the clause's examples write it
//!   (`?pick=temperature,humidity`); its braces are not URI characters and are escaped;
//! - a language filter (`lang`, clause 4.15) keeps its syntax `,`, `;`, `=` and `*` literal, all sub-delimiters RFC
//!   3986 section 3.4 allows in a query, as clause 4.15 example 4 writes it (`fr-CH,fr;q=0.9,*;q=0.5`);
//! - an entity ordering (`orderBy`, clause 4.23.3) keeps its separators `,` and `;` literal, as the clause's
//!   examples write them (`?orderBy=name,age;desc`); the brackets of a member path are not allowed in a query
//!   (RFC 3986 section 3.4) and are escaped;
//! - a geo-relationship (`georel`, clause 4.10) keeps its `;` and `=` literal, sub-delimiters RFC 3986 section 3.4
//!   allows in a query, as clause 4.10 example 1 writes it (`?georel=near;maxDistance==2000`);
//! - a scope query (`scopeQ`, clause 4.19) keeps `/`, `;`, `(` and `)` literal. Its `+` is escaped, since form
//!   decoding reads a literal `+` as a space, its `#` would end the query, and its `|` is not a URI character.

use crate::{
    http::QueryParameter,
    query::{geoquery::GeoRelation, language_filter::LanguageFilter, ordering::OrderBy, projection::Projection, scope_query::ScopeQuery},
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use std::fmt::Display;

/// Everything but the unreserved characters of RFC 3986 section 2.3.
const OPAQUE: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// An entity type selection: its sub-delimiter syntax `,`, `;`, `(` and `)`, and the `*` of every type, stay
/// literal.
const TYPE_SELECTION: &AsciiSet = &OPAQUE.remove(b',').remove(b';').remove(b'(').remove(b')').remove(b'*');

/// A projection: its list separator `,` stays literal.
const PROJECTION: &AsciiSet = &OPAQUE.remove(b',');

/// A language filter: its sub-delimiter syntax `,`, `;`, `=` and `*` stays literal.
const LANGUAGE_FILTER: &AsciiSet = &OPAQUE.remove(b',').remove(b';').remove(b'=').remove(b'*');

/// An entity ordering: its separators `,` and `;` stay literal.
const ORDER_BY: &AsciiSet = &OPAQUE.remove(b',').remove(b';');

/// A geo-relationship: its sub-delimiter syntax `;` and `=` stays literal.
const GEO_RELATION: &AsciiSet = &OPAQUE.remove(b';').remove(b'=');

/// A scope query: its `/`, `;`, `(` and `)` stay literal.
const SCOPE_QUERY: &AsciiSet = &OPAQUE.remove(b'/').remove(b';').remove(b'(').remove(b')');

/// The parameters of a request, in the order they were added.
#[derive(Debug, Clone, PartialEq, Eq, Default, derive_more::Display)]
pub(crate) struct QueryString(String);

impl QueryString {
    /// Starts `name=`. Parameter names are spec tokens and need no encoding.
    fn start(&mut self, name: QueryParameter) {
        if !self.0.is_empty() {
            self.0.push('&');
        }
        self.0.push_str(<&'static str>::from(name));
        self.0.push('=');
    }

    /// Appends `printed`, escaping every character of `escaped`.
    fn write(&mut self, printed: &str, escaped: &'static AsciiSet) {
        self.0.extend(utf8_percent_encode(printed, escaped));
    }

    /// Adds `name=value`, escaping every character of `escaped` in the printed value.
    fn push_escaped(&mut self, name: QueryParameter, value: impl Display, escaped: &'static AsciiSet) {
        self.start(name);
        self.write(&value.to_string(), escaped);
    }

    /// Adds `name=value` for an opaque value, escaping every reserved character.
    pub(crate) fn push(&mut self, name: QueryParameter, value: impl Display) {
        self.push_escaped(name, value, OPAQUE);
    }

    /// Adds `name=value` for an opaque value, when there is one.
    pub(crate) fn push_some<T: Display>(&mut self, name: QueryParameter, value: Option<T>) {
        if let Some(value) = value {
            self.push(name, value);
        }
    }

    /// Adds `name=a,b,c` when the list is not empty: each item escaped, the separators literal.
    pub(crate) fn push_list<T: Display>(&mut self, name: QueryParameter, values: &[T]) {
        let Some((first, rest)) = values.split_first() else {
            return;
        };
        self.start(name);
        self.write(&first.to_string(), OPAQUE);
        for value in rest {
            self.0.push(',');
            self.write(&value.to_string(), OPAQUE);
        }
    }

    /// Adds `name=selection` for an entity type selection, or the `*` of every type, when there is one.
    pub(crate) fn push_type_selection(&mut self, name: QueryParameter, selection: Option<&(impl Display + ?Sized)>) {
        if let Some(selection) = selection {
            self.push_escaped(name, selection, TYPE_SELECTION);
        }
    }

    /// Adds `name=projection` for a projection, when there is one.
    pub(crate) fn push_projection(&mut self, name: QueryParameter, projection: Option<&Projection>) {
        if let Some(projection) = projection {
            self.push_escaped(name, projection, PROJECTION);
        }
    }

    /// Adds `name=filter` for a language filter, when there is one.
    pub(crate) fn push_language_filter(&mut self, name: QueryParameter, filter: Option<&LanguageFilter>) {
        if let Some(filter) = filter {
            self.push_escaped(name, filter, LANGUAGE_FILTER);
        }
    }

    /// Adds `name=terms` for an entity ordering.
    pub(crate) fn push_order_by(&mut self, name: QueryParameter, order_by: &OrderBy) {
        self.push_escaped(name, order_by, ORDER_BY);
    }

    /// Adds `name=relation` for a geo-relationship.
    pub(crate) fn push_geo_relation(&mut self, name: QueryParameter, relation: &GeoRelation) {
        self.push_escaped(name, relation, GEO_RELATION);
    }

    /// Adds `name=query` for a scope query, when there is one.
    pub(crate) fn push_scope_query(&mut self, name: QueryParameter, query: Option<&ScopeQuery>) {
        if let Some(query) = query {
            self.push_escaped(name, query, SCOPE_QUERY);
        }
    }

    /// Whether no parameter was added.
    pub(crate) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{QueryParameter, query_string::QueryString},
        query::{
            SelectorType,
            geoquery::GeoRelation,
            language_filter::LanguageFilter,
            ordering::OrderBy,
            projection::Projection,
            scope_query::ScopeQuery,
            type_selection::TypeSelection,
        },
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn a_q_value_escapes_every_reserved_character() {
        let mut query = QueryString::default();
        query.push(QueryParameter::Type, "Vehicle");
        query.push(QueryParameter::Q, r#"brandName!="Mercedes";speed>50|(a,b)"#);
        query.push(QueryParameter::Format, "simplified");
        assert_eq!(
            query.to_string(),
            "type=Vehicle&q=brandName%21%3D%22Mercedes%22%3Bspeed%3E50%7C%28a%2Cb%29&format=simplified"
        );
    }

    #[test]
    fn unreserved_characters_stay_and_a_plus_is_escaped() {
        let mut query = QueryString::default();
        query.push(QueryParameter::Q, "a-b.c_d~e+f g");
        assert_eq!(query.to_string(), "q=a-b.c_d~e%2Bf%20g");
    }

    #[test]
    fn a_list_writes_literal_separators_and_escapes_a_comma_inside_an_item() {
        let mut query = QueryString::default();
        query.push_list(QueryParameter::Id, &["urn:a", "urn:b,c"]);
        query.push_list::<&str>(QueryParameter::Drop, &[]);
        query.push_some(QueryParameter::Limit, Some(2));
        query.push_some::<u32>(QueryParameter::Offset, None);
        assert_eq!(query.to_string(), "id=urn%3Aa,urn%3Ab%2Cc&limit=2");
    }

    #[test]
    fn a_type_selection_keeps_its_sub_delimiter_syntax_and_escapes_the_pipe() {
        let selection: TypeSelection = "(Home;Vehicle)|https://example.org/Motorhome".parse().unwrap();
        let mut query = QueryString::default();
        query.push_type_selection(QueryParameter::Type, Some(&selection));
        query.push_type_selection(QueryParameter::Type, None::<&TypeSelection>);
        query.push_type_selection(QueryParameter::Type, Some(&SelectorType::All));
        assert_eq!(query.to_string(), "type=(Home;Vehicle)%7Chttps%3A%2F%2Fexample.org%2FMotorhome&type=*");
    }

    #[test]
    fn a_language_filter_keeps_its_sub_delimiter_syntax() {
        let lang: LanguageFilter = "fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5".parse().unwrap();
        let mut query = QueryString::default();
        query.push_language_filter(QueryParameter::Lang, Some(&lang));
        query.push_language_filter(QueryParameter::Lang, None);
        assert_eq!(query.to_string(), "lang=fr-CH,fr;q=0.9,en;q=0.8,*;q=0.5");
    }

    #[test]
    fn an_ordering_keeps_its_separators_and_escapes_its_brackets() {
        let order_by: OrderBy = "address[city],name.observedAt;desc".parse().unwrap();
        let mut query = QueryString::default();
        query.push_order_by(QueryParameter::OrderBy, &order_by);
        assert_eq!(query.to_string(), "orderBy=address%5Bcity%5D,name.observedAt;desc");
    }

    #[test]
    fn a_geo_relationship_keeps_its_sub_delimiter_syntax() {
        let relation: GeoRelation = "near;maxDistance==2000".parse().unwrap();
        let mut query = QueryString::default();
        query.push_geo_relation(QueryParameter::Georel, &relation);
        assert_eq!(query.to_string(), "georel=near;maxDistance==2000");
    }

    #[test]
    fn a_scope_query_keeps_its_slashes_and_grouping_and_escapes_plus_hash_and_pipe() {
        let mut query = QueryString::default();
        for text in ["(/Madrid/Districts;/CompanyA)|/CompanyB", "/Madrid/+/ParqueNorte", "/Madrid/Gardens/#", "/#"] {
            let scope_q: ScopeQuery = text.parse().unwrap();
            query.push_scope_query(QueryParameter::ScopeQ, Some(&scope_q));
        }
        query.push_scope_query(QueryParameter::ScopeQ, None);
        assert_eq!(
            query.to_string(),
            "scopeQ=(/Madrid/Districts;/CompanyA)%7C/CompanyB&scopeQ=/Madrid/%2B/ParqueNorte&scopeQ=/Madrid/Gardens/%23&scopeQ=/%23"
        );
    }

    #[test]
    fn a_projection_keeps_its_comma_and_escapes_its_braces() {
        let pick: Projection = "temperature,observation{humidity|pressure}".parse().unwrap();
        let mut query = QueryString::default();
        query.push_projection(QueryParameter::Pick, Some(&pick));
        query.push_projection(QueryParameter::Omit, None);
        assert_eq!(query.to_string(), "pick=temperature,observation%7Bhumidity,pressure%7D");
    }
}
