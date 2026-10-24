//! The parser of the NGSI-LD Query Language (the ABNF of NGSI-LD clause 4.9).
//!
//! It accepts what the examples of clause 4.9 use beyond the ABNF; the module documentation of
//! [`q`](crate::query::q) lists the differences.
//!
//! Clause 4.9 bounds neither how deep groups nest nor how many relationships a path follows, and a broker returns
//! queries this parser reads. A linked entity path is read in a loop; a group is read on a stack that grows with
//! the depth ([`grow`]), so a query nested any number of levels deep is read without overflowing the stack.

use crate::{
    model::{InvalidJson, Number, TemporalError, Term, Uri, nesting::grow},
    query::{
        Expected,
        ParseError,
        grammar::{Failure, Input, Outcome, expecting, term},
        q::{
            AttributePath,
            ComparableValue,
            Comparison,
            EqualityValue,
            Link,
            MemberName,
            Operator,
            Pattern,
            Query,
            QueryTerm,
            Range,
            UnquotedUri,
            Value,
            ValueList,
            ValuePath,
            path::is_member_char,
            value::ends_unquoted_uri,
        },
        regular_expression::{Boundary, extent},
    },
};
use jstrict::{NumberBuf, parse::parse_str_value};
use std::str::FromStr;
use winnow::{
    Parser,
    combinator::{alt, cut_err, eof, opt, peek, preceded, repeat, terminated},
    error::{ErrMode, ParserError},
    stream::{Location, Stream},
    token::{any, none_of, one_of, take_till, take_while},
};

/// Parses a query: conjunctions separated by `|`.
pub(crate) fn query(input: &mut Input<'_>) -> Outcome<Query> {
    let first = conjunction(input)?;
    let rest: Vec<Query> = repeat(0.., preceded('|', cut_err(conjunction))).parse_next(input)?;
    Ok(rest.into_iter().fold(first, Query::or))
}

/// Parses operands separated by `;`, which binds tighter than `|` (clause 4.9).
fn conjunction(input: &mut Input<'_>) -> Outcome<Query> {
    let first = operand(input)?;
    let rest: Vec<Query> = repeat(0.., preceded(';', cut_err(operand))).parse_next(input)?;
    Ok(rest.into_iter().fold(first, Query::and))
}

/// Parses a parenthesised query or a query term.
fn operand(input: &mut Input<'_>) -> Outcome<Query> {
    expecting(Expected::QueryTerm, alt((group, query_term.map(Query::Term)))).parse_next(input)
}

/// Parses a parenthesised query (`QueryTermAssoc`), nested to any depth as in clause 4.9 example 6.
fn group(input: &mut Input<'_>) -> Outcome<Query> {
    let position = input.current_token_start();
    '('.parse_next(input)?;
    let inner = cut_err(|input: &mut Input<'_>| grow(|| query(input))).parse_next(input)?;
    cut_err(')')
        .parse_next(input)
        .map_err(|_: ErrMode<Failure>| Failure::fatal(ParseError::UnbalancedParenthesis { position }))?;
    Ok(inner)
}

/// Parses a query term: an attribute path, optionally followed by a comparison.
fn query_term(input: &mut Input<'_>) -> Outcome<QueryTerm> {
    let path = attribute_path(input)?;
    let comparison = opt(comparison).parse_next(input)?;
    Ok(QueryTerm { path, comparison })
}

/// Parses an attribute path: a value path, after any number of relationships each opening a linked entity path
/// in braces.
fn attribute_path(input: &mut Input<'_>) -> Outcome<AttributePath> {
    let mut links = Vec::new();
    let mut name = term(input)?;
    while opt('{').parse_next(input)?.is_some() {
        let entity_types = opt(terminated(entity_types, ':')).parse_next(input)?.unwrap_or_default();
        links.push(Link::new(name).with_entity_types(entity_types));
        name = cut_err(expecting(Expected::AttributeName, term)).parse_next(input)?;
    }
    let target = value_path(name, input)?;
    links
        .iter()
        .try_for_each(|_| cut_err(expecting(Expected::ClosingBrace, '}')).void().parse_next(input))?;
    Ok(AttributePath::through(links, target))
}

/// Parses the entity type hint of a linked entity path, without its `:`.
fn entity_types(input: &mut Input<'_>) -> Outcome<Vec<Term>> {
    let first = term(input)?;
    let rest: Vec<Term> = repeat(0.., preceded(',', term)).parse_next(input)?;
    Ok([first].into_iter().chain(rest).collect())
}

/// Parses a whole value path (the `ValuePath` production), which the ordering language of clause 4.23.3 reuses.
pub(crate) fn whole_value_path(input: &mut Input<'_>) -> Outcome<ValuePath> {
    let first = term(input)?;
    value_path(first, input)
}

/// Parses the rest of a value path whose first name is `first`: dotted names, then an optional member path.
fn value_path(first: Term, input: &mut Input<'_>) -> Outcome<ValuePath> {
    let names: Vec<Term> = repeat(0.., preceded('.', cut_err(expecting(Expected::AttributeName, term)))).parse_next(input)?;
    let path = names.into_iter().fold(ValuePath::new(first), ValuePath::with_sub_attribute);
    if opt('[').parse_next(input)?.is_none() {
        return Ok(path);
    }
    let first_member = cut_err(expecting(Expected::Member, member)).parse_next(input)?;
    let members: Vec<MemberName> = repeat(0.., preceded('.', cut_err(expecting(Expected::Member, member)))).parse_next(input)?;
    cut_err(expecting(Expected::ClosingBracket, ']')).parse_next(input)?;
    Ok([first_member].into_iter().chain(members).fold(path, ValuePath::with_member))
}

/// Parses one member of a member path.
fn member(input: &mut Input<'_>) -> Outcome<MemberName> {
    let position = input.current_token_start();
    let text = take_while(1.., |c: char| is_member_char(c) || c == '*').parse_next(input)?;
    MemberName::try_new(text).map_err(|source| Failure::fatal(ParseError::InvalidMember { position, source }))
}

/// The characters operators are made of.
const OPERATOR_CHARACTERS: [char; 5] = ['=', '!', '<', '>', '~'];

/// Parses an operator and the value it takes.
fn comparison(input: &mut Input<'_>) -> Outcome<Comparison> {
    let position = input.current_token_start();
    // No operator character means no comparison (a bare path tests existence); once one is there, the operator is
    // the longest one that matches, so a pattern that starts with an operator character (`name~==x` is `~=`
    // followed by the pattern `=x`) reads back as it was printed.
    peek(one_of(OPERATOR_CHARACTERS)).parse_next(input)?;
    let token = cut_err(expecting(Expected::Operator, alt(("!~=", "~=", "==", "!=", ">=", "<=", ">", "<")))).parse_next(input)?;
    let operator = Operator::from_str(token).map_err(|_| Failure::expected(Expected::Operator, position).cut())?;
    match operator {
        Operator::Equal => cut_err(equality_value).map(Comparison::Equal).parse_next(input),
        Operator::Unequal => cut_err(equality_value).map(Comparison::Unequal).parse_next(input),
        Operator::Greater => ordered_value(input).map(Comparison::Greater),
        Operator::GreaterOrEqual => ordered_value(input).map(Comparison::GreaterOrEqual),
        Operator::Less => ordered_value(input).map(Comparison::Less),
        Operator::LessOrEqual => ordered_value(input).map(Comparison::LessOrEqual),
        Operator::Matches => cut_err(pattern).map(Comparison::Matches).parse_next(input),
        Operator::NotMatches => cut_err(pattern).map(Comparison::NotMatches).parse_next(input),
    }
}

/// Parses the comparable value an ordering operator takes.
fn ordered_value(input: &mut Input<'_>) -> Outcome<ComparableValue> {
    cut_err(expecting(Expected::ComparableValue, comparable_value)).parse_next(input)
}

/// Parses the value of `==` or `!=`: a value, a value list or a range ending the term, or else an unquoted URI.
fn equality_value(input: &mut Input<'_>) -> Outcome<EqualityValue> {
    expecting(Expected::Value, alt((terminated(literal, term_end), unquoted_uri.map(EqualityValue::Uri)))).parse_next(input)
}

/// Succeeds, without consuming anything, where a query term may end.
fn term_end(input: &mut Input<'_>) -> Outcome<()> {
    peek(alt((one_of(ends_unquoted_uri).void(), eof.void()))).parse_next(input)
}

/// Parses a single value, a value list or a range.
fn literal(input: &mut Input<'_>) -> Outcome<EqualityValue> {
    let first = match value(input)? {
        Value::Comparable(min) => {
            if opt("..").parse_next(input)?.is_some() {
                let max = cut_err(expecting(Expected::ComparableValue, comparable_value)).parse_next(input)?;
                return Ok(EqualityValue::Range(Range::new(min, max)));
            }
            Value::Comparable(min)
        }
        boolean @ Value::Boolean(_) => boolean,
    };
    let rest: Vec<Value> = repeat(0.., preceded(',', cut_err(expecting(Expected::Value, value)))).parse_next(input)?;
    if rest.is_empty() {
        return Ok(EqualityValue::Single(first));
    }
    let values = [first].into_iter().chain(rest).collect::<Vec<_>>();
    ValueList::try_from(values).map(EqualityValue::List).map_err(|_| ErrMode::from_input(input))
}

/// Parses a value of a value list: a boolean or a comparable value.
fn value(input: &mut Input<'_>) -> Outcome<Value> {
    alt((
        "true".value(Value::Boolean(true)),
        "false".value(Value::Boolean(false)),
        comparable_value.map(Value::Comparable),
    ))
    .parse_next(input)
}

/// Parses a comparable value. Temporal values come first: a number would read their leading digits.
fn comparable_value(input: &mut Input<'_>) -> Outcome<ComparableValue> {
    alt((
        temporal(date_time_shape).map(ComparableValue::DateTime),
        temporal(date_shape).map(ComparableValue::Date),
        temporal(time_shape).map(ComparableValue::Time),
        number.map(ComparableValue::Number),
        string.map(ComparableValue::Text),
    ))
    .parse_next(input)
}

/// Recognises exactly `count` ASCII digits.
fn digits<'i>(count: usize) -> impl Parser<Input<'i>, &'i str, ErrMode<Failure>> {
    take_while(count, |c: char| c.is_ascii_digit())
}

/// Recognises the shape of a Date: `YYYY-MM-DD`.
fn date_shape<'i>(input: &mut Input<'i>) -> Outcome<&'i str> {
    (digits(4), '-', digits(2), '-', digits(2)).take().parse_next(input)
}

/// Recognises the shape of a Time: `hh:mm:ss[.s+]Z`, with `,` accepted for `.` (clause 4.6.3).
fn time_shape<'i>(input: &mut Input<'i>) -> Outcome<&'i str> {
    let fraction = (one_of(['.', ',']), take_while(1.., |c: char| c.is_ascii_digit()));
    (digits(2), ':', digits(2), ':', digits(2), opt(fraction), 'Z').take().parse_next(input)
}

/// Recognises the shape of a `DateTime`: a Date, `T` and a Time.
fn date_time_shape<'i>(input: &mut Input<'i>) -> Outcome<&'i str> {
    (date_shape, 'T', time_shape).take().parse_next(input)
}

/// Parses a temporal value whose text `shape` recognises; a text of the right shape that is not a valid value
/// is an error, not a different kind of value.
fn temporal<'i, T: FromStr<Err = TemporalError>>(mut shape: impl Parser<Input<'i>, &'i str, ErrMode<Failure>>) -> impl Parser<Input<'i>, T, ErrMode<Failure>> {
    move |input: &mut Input<'i>| {
        let position = input.current_token_start();
        let text = shape.parse_next(input)?;
        text.parse().map_err(|source| Failure::fatal(ParseError::InvalidTemporal { position, source }))
    }
}

/// Parses a JSON number (IETF RFC 8259, section 6), keeping its lexical form.
fn number(input: &mut Input<'_>) -> Outcome<Number> {
    let digits = || take_while(1.., |c: char| c.is_ascii_digit());
    let integer = alt(('0'.void(), (one_of('1'..='9'), take_while(0.., |c: char| c.is_ascii_digit())).void()));
    let fraction = ('.', digits());
    let exponent = (one_of(['e', 'E']), opt(one_of(['+', '-'])), digits());
    let text = (opt('-'), integer, opt(fraction), opt(exponent)).take().parse_next(input)?;
    NumberBuf::from_str(text).map(Number::from_json).map_err(|_| ErrMode::from_input(input))
}

/// Parses a quoted JSON string (IETF RFC 8259, section 7), unescaping it.
fn string(input: &mut Input<'_>) -> Outcome<String> {
    let position = input.current_token_start();
    let character = alt((('\\', any).void(), none_of(['"', '\\']).void()));
    let body = repeat::<_, _, (), _, _>(0.., character);
    let text = ('"', body, cut_err(expecting(Expected::ClosingQuote, '"'))).take().parse_next(input)?;
    let json = parse_str_value(text).map_err(|error| {
        Failure::fatal(ParseError::InvalidString {
            position,
            source: InvalidJson::of(&error),
        })
    })?;
    json.into_string().map(String::from).ok_or_else(|| Failure::expected(Expected::Value, position))
}

/// Parses an unquoted URI, which ends at the first `;`, `|` or `)`.
fn unquoted_uri(input: &mut Input<'_>) -> Outcome<UnquotedUri> {
    let text = take_till(1.., ends_unquoted_uri).parse_next(input)?;
    text.parse::<Uri>()
        .ok()
        .and_then(|uri| UnquotedUri::try_from(uri).ok())
        .ok_or_else(|| ErrMode::from_input(input))
}

/// Parses a regular expression, which ends where [`extent`] says a query ends it.
fn pattern(input: &mut Input<'_>) -> Outcome<Pattern> {
    let position = input.current_token_start();
    let length = extent(**input, Boundary::Query).length;
    if length == 0 {
        return Err(Failure::expected(Expected::Pattern, position));
    }
    let text = input.next_slice(length);
    Pattern::try_new(text).map_err(|source| Failure::fatal(ParseError::InvalidPattern { position, source }))
}
