# Changelog

All notable changes to this project will be documented in this file.

## [v1.3.1] - 2026-10-31

### ⚡ Performance
- `model` - check instance dataset ids and instance ids in constant time

### 🧹 Chores
- bump version

## [v1.3.0] - 2026-10-30

### 🚀 Features
- `model` - read a lone attribute body with its @context
- `http` - request the first and last pages a page links
- `query` - convert an entity type name into a selector type builder
- `model` - convert an entity type name into a term
- `client` - take entity selectors when creating an entity map

### 🐛 Bug Fixes
- `http` - refuse a fragment naming an entity other than its target
- `model` - read a value object typed with the term type

### 📚 Documentation
- `model` - say why value is exhaustive

### 🧪 Tests
- `model` - read a json literal under an expanded term by path
- `http` - read a 207 naming the scope by its fully qualified name
- `model` - read a non-null json literal in a property value as an object
- `subscription` - read previousobject of a relationship deleted without deletedat
- `http` - read a keyvalues page answering an unchecked request
- pin the typed inputs of 1.2.0 over the wire
- `client` - send replace entity with and without its id over the wire
- `model` - pin a lone sub-attribute and a typeless concise payload in partial attribute update
- `subscription` - pin the keyvalues notification format on update and over the wire
- `client` - send a batch upsert with and without options over the wire
- `client` - send each body of partial attribute update over the wire

### 🧹 Chores
- bump version

## [v1.2.0] - 2026-10-30

### 🚀 Features
- `model` - read objecttype as a term ⚠️ **BREAKING**
- `model` - hold the json null literal as a property value ⚠️ **BREAKING**
- `model,http` - let a value keep the spelling it was built with
- `model` - let a temporal request record the removal of a scope
- `http,model` - keep the body of a 406 and build positions from exact numbers
- `http` - let add attributes to temporal evolution name its target
- `http` - name several entity selectors in temporal queries and entity map creations
- `client` - build an unchecked request aimed at a snapshot
- `model` - add as_str to the name types
- `query` - convert an entity type name into a type selection builder
- `model` - convert a boolean, number, object or typed value into a value
- `subscription` - accept csf on a context source registration subscription ⚠️ **BREAKING**
- `http` - report the success status a page or retrieval was answered with
- `http` - read the query strings of the registration, subscription, context and discovery listings
- `registration` - read the body of update entity map
- `model` - read the fragments, the clone body and the temporal bodies the crate only wrote
- `http` - let delete attribute name a dataset by its datasetid ⚠️ **BREAKING**
- `model` - let a temporal evolution record a deleted instance in a request
- `model` - let partial attribute update send a fragment that sets no value ⚠️ **BREAKING**
- `registration` - let a registration fragment leave its type out
- `http` - let replace entity leave the id out of its body
- `http` - let a batch upsert send options=replace ⚠️ **BREAKING**
- `http` - let a deletion of attribute instances send deleteall=false ⚠️ **BREAKING**
- `subscription` - let a client send keyvalues as the notification format ⚠️ **BREAKING**
- `evolution` - let a client choose the single-object spelling of a temporal evolution ⚠️ **BREAKING**

### 🐛 Bug Fixes
- `http` - refuse an update result that leaves out a mandatory list
- `model` - read the keywords @id and @type of a request as id and type
- `http` - read an ordering parameter given without orderby as asking for no ordering ⚠️ **BREAKING**
- `model` - read a null optional member of stored context metadata as left out
- `model` - read an attribute with no instance in a temporal response
- `http` - read a comma encoded in a list of names or tokens as the separator
- `http` - let a purge limited by the attributes it drops or keeps be sent

### 🧪 Tests
- `http` - pin a conditional serve of a stored context and its 304
- `http` - pin the unchecked road for requests the specification does not define
- `http` - cover projected entities in the temporal operations, the query by post and paging
- `http` - derive deliberately invalid subscription and registration bodies from their typed form
- `http` - cover the context of a replace attribute body
- `subscription` - cover mqtt and mqtts notification endpoints
- `http` - pin that an opaque page continuation is followed verbatim and not read as a parameter
- `model` - pin a language property with several strings for one language

### 🧹 Chores
- bump version

## [v1.1.0] - 2026-10-24

### 🚀 Features
- add a way to obtain raw response body

## [v1.0.0] - 2026-10-24

### 🚀 Features
- enable the reqwest client with rustls by default ⚠️ **BREAKING**
- add the ngsi-ld data model, query languages, http binding and client

### 🐛 Bug Fixes
- gate the linked-data imports of context source notifications

### 🔧 Refactor
- `model` - seal the hooks of the model's sealed traits ⚠️ **BREAKING**
- `api` - prefix the remaining fallible constructors with try_ ⚠️ **BREAKING**
- `api` - prefix every fallible constructor with try_ ⚠️ **BREAKING**
- rename the temporal query operation builder ⚠️ **BREAKING**
- `api` - seal the hooks of sealed traits ⚠️ **BREAKING**
- `api` - keep jstrict error types out of the public api ⚠️ **BREAKING**

### 📚 Documentation
- `client` - build the module indexes with any feature set
- shorten every rustdoc summary to one plain sentence
- format every rustdoc example and wrap the docs at 120 columns
- shorten the live testing guide
- cleanup
- fix stale and internal references in the public docs
- update code of conduct
- unify terminology
- state that only conformant brokers are supported
- add the readme, contributing guide and project docs

### 🧪 Tests
- add the opt-in live tests against a running broker
- add the integration tests and examples

### 🏗️ Build
- `deps` - bump rust-toolchain from 1.99.0 to 1.99.1
- release as 1.0.0 ⚠️ **BREAKING**
- disable default features only where they add something unused
- set the msrv to 1.96.0

### 👷 CI
- pin the cargo-machete action to a release
- prettify issue templates
- run only a quick check on dependabot pull requests
- run only the checks a pull request can affect
- update dependencies, the rust toolchain and actions daily without grouping
- publish to crates.io and create the github release on a version tag

### 🧹 Chores
- update dependencies
- add issue forms and a pull request template
- set-up repository
- init

<!-- generated by git-cliff -->
