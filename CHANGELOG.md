# Changelog

## [2.0.1](https://github.com/mskvarc/ngsi-ld/compare/v2.0.0...v2.0.1) - 2026-11-09

### Changes

#### 📚 Documentation

- link the andromeda quickstart

#### 🧹 Chores

- bump version
- update dependencies

## [2.0.0](https://github.com/mskvarc/ngsi-ld/compare/v1.3.1...v2.0.0) - 2026-11-06

### Changes

#### ⚠️ Breaking changes

- require iri-rs 4 and jsonld 0.27

#### 🚀 Features

- **http:** page temporal attribute instances with firstn, offsetn and interval links
- **http:** serve a stored context with the core context of a version
- **http:** set, remove or add query parameters of an unchecked request
- **http:** send an unchecked scope query as written
- **http:** add set attribute value operation
- **http:** add retrieve attribute value operation
- **http:** add retrieve attribute operation
- **http:** add head requests for read operations

#### 🐛 Bug Fixes

- compare uris as rfc 3986 section 6 recommends

#### 🧹 Chores

- bump version

## [1.3.1](https://github.com/mskvarc/ngsi-ld/compare/v1.3.0...v1.3.1) - 2026-10-31

### Changes

#### ⚡ Performance

- **model:** check instance dataset ids and instance ids in constant time

#### 🧹 Chores

- bump version

## [1.3.0](https://github.com/mskvarc/ngsi-ld/compare/v1.2.0...v1.3.0) - 2026-10-30

### Changes

#### 🚀 Features

- **model:** read a lone attribute body with its `@context`
- **http:** request the first and last pages a page links
- **query:** convert an entity type name into a selector type builder
- **model:** convert an entity type name into a term
- **client:** take entity selectors when creating an entity map

#### 🐛 Bug Fixes

- **http:** refuse a fragment naming an entity other than its target
- **model:** read a value object typed with the term type

#### 📚 Documentation

- **model:** say why value is exhaustive

#### 🧪 Tests

- **model:** read a json literal under an expanded term by path
- **http:** read a 207 naming the scope by its fully qualified name
- **model:** read a non-null json literal in a property value as an object
- **subscription:** read previousobject of a relationship deleted without deletedat
- **http:** read a keyvalues page answering an unchecked request
- pin the typed inputs of 1.2.0 over the wire
- **client:** send replace entity with and without its id over the wire
- **model:** pin a lone sub-attribute and a typeless concise payload in partial attribute update
- **subscription:** pin the keyvalues notification format on update and over the wire
- **client:** send a batch upsert with and without options over the wire
- **client:** send each body of partial attribute update over the wire

#### 🧹 Chores

- bump version

## [1.2.0](https://github.com/mskvarc/ngsi-ld/compare/v1.1.0...v1.2.0) - 2026-10-30

### Changes

#### ⚠️ Breaking changes

- **model:** read objecttype as a term
- **model:** hold the json null literal as a property value
- **subscription:** accept csf on a context source registration subscription. the compile-fail guarantee that csf cannot be set on a registration subscription builder is gone; EntitySubscriptionMembers and EntitySubscriptionChanges no longer hold csf.
- **http:** read an ordering parameter given without orderby as asking for no ordering. ParameterError::OrderingWithoutOrderBy no longer exists.
- **http:** let delete attribute name a dataset by its datasetid. InstanceSelection::Dataset, InstanceSelection::Selected and DeleteAttributeParameters::dataset_id hold a DatasetId instead of a Uri.
- **model:** let partial attribute update send a fragment that sets no value. the body type of PartialAttributeUpdate and of the client's update_attribute is Change<AttributeFragment, InstanceDeletion>, and Change::<Attribute<Patch>, InstanceDeletion>::parse is now Change::<AttributeFragment, InstanceDeletion>::parse.
- **http:** let a batch upsert send options=replace. UpsertMode gains the Unspecified variant, which replaces Replace as the default, and Replace now writes options=replace.
- **http:** let a deletion of attribute instances send deleteall=false. InstanceSelection is exhaustive, so a match over it needs the new arm.
- **subscription:** let a client send keyvalues as the notification format. SimplifiedSpelling is now ngsi_ld::model::SimplifiedSpelling; ngsi_ld::http::operation::SimplifiedSpelling is removed. Two NotificationParams that differ only in the name of the simplified format are no longer equal.
- **evolution:** let a client choose the single-object spelling of a temporal evolution. CreateOrUpdateTemporalEvolution has a new public field, so code that builds it with a struct literal must set spelling. Code that uses its builder is unaffected.

#### 🚀 Features

- **model,http:** let a value keep the spelling it was built with
- **model:** let a temporal request record the removal of a scope
- **http,model:** keep the body of a 406 and build positions from exact numbers
- **http:** let add attributes to temporal evolution name its target
- **http:** name several entity selectors in temporal queries and entity map creations
- **client:** build an unchecked request aimed at a snapshot
- **model:** add as_str to the name types
- **query:** convert an entity type name into a type selection builder
- **model:** convert a boolean, number, object or typed value into a value
- **http:** report the success status a page or retrieval was answered with
- **http:** read the query strings of the registration, subscription, context and discovery listings
- **registration:** read the body of update entity map
- **model:** read the fragments, the clone body and the temporal bodies the crate only wrote
- **model:** let a temporal evolution record a deleted instance in a request
- **registration:** let a registration fragment leave its type out
- **http:** let replace entity leave the id out of its body

#### 🐛 Bug Fixes

- **http:** refuse an update result that leaves out a mandatory list
- **model:** read the keywords `@id` and `@type` of a request as id and type
- **model:** read a null optional member of stored context metadata as left out
- **model:** read an attribute with no instance in a temporal response
- **http:** read a comma encoded in a list of names or tokens as the separator
- **http:** let a purge limited by the attributes it drops or keeps be sent

#### 🧪 Tests

- **http:** pin a conditional serve of a stored context and its 304
- **http:** pin the unchecked road for requests the specification does not define
- **http:** cover projected entities in the temporal operations, the query by post and paging
- **http:** derive deliberately invalid subscription and registration bodies from their typed form
- **http:** cover the context of a replace attribute body
- **subscription:** cover mqtt and mqtts notification endpoints
- **http:** pin that an opaque page continuation is followed verbatim and not read as a parameter
- **model:** pin a language property with several strings for one language

#### 🧹 Chores

- bump version

## [1.1.0](https://github.com/mskvarc/ngsi-ld/compare/v1.0.0...v1.1.0) - 2026-10-24

### Changes

#### 🚀 Features

- add a way to obtain raw response body

## [1.0.0](https://github.com/mskvarc/ngsi-ld/releases/tag/v1.0.0) - 2026-10-24

Initial release.

<!-- generated by git-cliff -->
