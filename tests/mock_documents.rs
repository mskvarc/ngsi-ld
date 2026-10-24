//! The documents a broker or a context source sends, built and read in typed form, as a mock broker, context source
//! or notification receiver under test does: output entities and attributes (clause 5.2.2), temporal evolutions
//! (clauses 4.5.7 to 4.5.9 and 4.5.19), notifications (clause 5.3), the fragments a broker forwards (clause 5.4), and
//! inline joins as deep as `joinLevel` asks (clause 4.5.23.2).

#[cfg(test)]
mod mock_documents {
    mod deep_nesting;
    mod fragments;
    mod linked_entities;
    mod notifications;
    mod output_entities;
    mod temporal_input;
    mod temporal_output;
}
