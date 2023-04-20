use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
use std::collections::BTreeMap;

/// Instantiate message
#[cw_serde]
pub struct InstantiateMsg {
    /// Limitations regarding store usage.
    pub limits: StoreLimits,
}

/// Execute messages
#[cw_serde]
pub enum ExecuteMsg {
    /// # Insert
    /// Insert the data as RDF triples in the store.
    /// For already existing triples it acts as no-op.
    ///
    /// Only the smart contract owner (i.e. the address who instantiated it) is authorized to perform
    /// this action.
    InsertData { input: DataInput },
}

/// # SelectQuery
/// Query messages
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// # Resources
    ///
    /// Returns the resources matching the criteria defined by the provided query.
    ///
    #[returns(SelectResponse)]
    Select {
        /// The query to execute.
        query: SelectQuery,
    },
}

/// # StoreLimits
/// Contains limitations regarding store usages.
#[cw_serde]
pub struct StoreLimits {
    /// The maximum number of triples the store can contains.
    /// If `None`, there is no limit on the number of triples.
    pub max_triple_count: Option<Uint128>,
    /// The maximum number of bytes the store can contains.
    /// The size of a triple is counted as the sum of the size of its subject, predicate and object,
    /// including the size of data types and language tags if any.
    /// If `None`, there is no limit on the number of bytes.
    pub max_byte_size: Option<Uint128>,
    /// The maximum number of bytes the store can contains for a single triple.
    /// The size of a triple is counted as the sum of the size of its subject, predicate and object,
    /// including the size of data types and language tags if any. The limit is used to prevent
    /// storing very large triples, especially literals.
    /// If `None`, there is no limit on the number of bytes.
    pub max_triple_byte_size: Option<Uint128>,
    /// The maximum limit of a query, i.e. the maximum number of triples returned by a select query.
    /// If `None`, there is no limit on the number of triples returned.
    pub max_query_limit: Option<Uint128>,
    /// The maximum number of variables a query can select.
    /// If `None`, there is no limit on the number of variables.
    pub max_query_variable_count: Option<Uint128>,
    /// The maximum number of bytes an insert data query can contains.
    /// If `None`, there is no limit on the number of bytes.
    pub max_insert_data_byte_size: Option<Uint128>,
    /// The maximum number of triples an insert data query can contains (after parsing).
    /// If `None`, there is no limit on the number of triples.
    pub max_insert_data_triple_count: Option<Uint128>,
}

/// # DataInput
/// Represents the input data for the [ExecuteMsg::InsertData] message as RDF triples in a specific format.
#[cw_serde]
pub enum DataInput {
    /// # RDF XML
    /// Input in [RDF/XML](https://www.w3.org/TR/rdf-syntax-grammar/) format.
    #[serde(rename = "rdf_xml")]
    RDFXml(Binary),
    /// # Turtle
    /// Input in [Turtle](https://www.w3.org/TR/turtle/) format.
    Turtle(Binary),
    /// # N-Triples
    /// Input in [N-Triples](https://www.w3.org/TR/n-triples/) format.
    NTriples(Binary),
}

/// # IRI
/// Represents an IRI.
#[cw_serde]
pub enum IRI {
    /// # Prefixed
    /// An IRI prefixed with a prefix.
    /// The prefixed IRI is expanded to a full IRI using the prefix definition specified in the query.
    /// For example, the prefixed IRI `rdf:type` is expanded to `http://www.w3.org/1999/02/22-rdf-syntax-ns#type`.
    Prefixed(String),
    /// # Full
    /// A full IRI.
    Full(String),
}

/// # SelectResponse
/// Represents the response of a [QueryMsg::Select] query.
#[cw_serde]
pub struct SelectResponse {
    /// The head of the response, i.e. the set of variables mentioned in the results.
    pub head: Head,
    /// The results of the select query.
    pub results: Results,
}

/// # Head
/// Represents the head of a [SelectResponse].
#[cw_serde]
pub struct Head {
    /// The variables selected in the query.
    pub vars: Vec<String>,
}

/// # Results
/// Represents the results of a [SelectResponse].
#[cw_serde]
pub struct Results {
    /// The bindings of the results.
    pub bindings: Vec<BTreeMap<String, Value>>,
}

/// # Value
#[cw_serde]
#[serde(tag = "type")]
pub enum Value {
    /// # URI
    /// Represents an IRI.
    URI {
        /// The value of the IRI.
        value: IRI,
    },
    /// # Literal
    /// Represents a literal S with optional language tag L or datatype IRI D.
    Literal {
        /// The value of the literal.
        value: String,
        /// The language tag of the literal.
        #[serde(rename = "xml:lang")]
        lang: Option<String>,
        /// The datatype of the literal.
        datatype: Option<IRI>,
    },
    /// # BlankNode
    /// Represents a blank node.
    BlankNode {
        /// The identifier of the blank node.
        value: String,
    },
}

/// # SelectQuery
/// Represents a SELECT query over the triple store, allowing to select variables to return
/// and to filter the results.
#[cw_serde]
pub struct SelectQuery {
    /// The prefixes used in the query.
    pub prefixes: Vec<Prefix>,
    /// The items to select.
    /// Note: the number of items to select cannot exceed the maximum query variable count defined
    /// in the store limitations.
    pub select: Vec<SelectItem>,
    /// The WHERE clause.
    /// If `None`, there is no WHERE clause, i.e. all triples are returned without filtering.
    pub r#where: WhereClause,
    /// The maximum number of results to return.
    /// If `None`, there is no limit.
    /// Note: the value of the limit cannot exceed the maximum query limit defined in the store
    /// limitations.
    pub limit: Option<u64>,
}

/// # Prefix
/// Represents a prefix in a [SelectQuery]. A prefix is a shortcut for a namespace used in the query.
#[cw_serde]
pub struct Prefix {
    /// The prefix.
    pub prefix: String,
    /// The namespace associated with the prefix.
    pub namespace: String,
}

/// # SelectItem
/// Represents an item to select in a [SelectQuery].
#[cw_serde]
pub enum SelectItem {
    /// # Variable
    /// Represents a variable.
    Variable(String),
}

/// # WhereClause
/// Represents a WHERE clause in a [SelectQuery], i.e. a set of conditions to filter the results.
pub type WhereClause = Vec<WhereCondition>;

/// # WhereCondition
/// Represents a condition in a [WhereClause].
#[cw_serde]
pub enum WhereCondition {
    /// # Simple
    /// Represents a simple condition.
    Simple(SimpleWhereCondition),
}

/// # SimpleWhereCondition
/// Represents a simple condition in a [WhereCondition].
#[cw_serde]
pub enum SimpleWhereCondition {
    /// # TriplePattern
    /// Represents a triple pattern, i.e. a condition on a triple based on its subject, predicate and
    /// object.
    TriplePattern(TriplePattern),
}

/// # TriplePattern
/// Represents a triple pattern in a [SimpleWhereCondition].
#[cw_serde]
pub struct TriplePattern {
    /// The subject of the triple pattern.
    pub subject: SubjectPattern,
    /// The predicate of the triple pattern.
    pub predicate: PredicatePattern,
    /// The object of the triple pattern.
    pub object: ObjectPattern,
}

/// # SubjectPattern
/// Represents a subject pattern in a [TriplePattern] that can be either a variable or a node.
#[cw_serde]
pub enum SubjectPattern {
    /// # Variable
    /// A variable.
    Variable(String),
    /// # Node
    /// A node, i.e. an IRI or a blank node.
    Node(Node),
}

/// # PredicatePattern
/// Represents a predicate pattern in a [TriplePattern] that can be either a variable or a node.
#[cw_serde]
pub enum PredicatePattern {
    /// # Variable
    /// A variable.
    Variable(String),
    /// # Node
    /// A node, i.e. an IRI or a blank node.
    Node(Node),
}

/// # ObjectPattern
/// Represents an object pattern in a [TriplePattern] that can be either a variable, a node or a literal.
#[cw_serde]
pub enum ObjectPattern {
    /// # Variable
    /// A variable.
    Variable(String),
    /// # Node
    /// A node, i.e. an IRI or a blank node.
    Node(Node),
    /// # Literal
    /// An RDF [literal](https://www.w3.org/TR/rdf11-concepts/#dfn-literal), i.e. a simple literal,
    /// a language-tagged string or a typed value.
    Literal(Literal),
}

/// # Literal
/// An RDF [literal](https://www.w3.org/TR/rdf11-concepts/#dfn-literal).
#[cw_serde]
pub enum Literal {
    /// # Simple
    /// A [simple literal](https://www.w3.org/TR/rdf11-concepts/#dfn-simple-literal) without datatype or language form.
    Simple(String),
    /// # LanguageTaggedString
    /// A [language-tagged string](https://www.w3.org/TR/rdf11-concepts/#dfn-language-tagged-string)
    LanguageTaggedString {
        /// The [lexical form](https://www.w3.org/TR/rdf11-concepts/#dfn-lexical-form).
        value: String,
        /// The [language tag](https://www.w3.org/TR/rdf11-concepts/#dfn-language-tag).
        language: String,
    },
    /// # TypedValue
    /// A value with a datatype.
    TypedValue {
        /// The [lexical form](https://www.w3.org/TR/rdf11-concepts/#dfn-lexical-form).
        value: String,
        /// The [datatype IRI](https://www.w3.org/TR/rdf11-concepts/#dfn-datatype-iri).
        datatype: IRI,
    },
}

/// # Node
#[cw_serde]
pub enum Node {
    /// # NamedNode
    /// An RDF [IRI](https://www.w3.org/TR/rdf11-concepts/#dfn-iri).
    NamedNode(IRI),
    /// # BlankNode
    /// An RDF [blank node](https://www.w3.org/TR/rdf11-concepts/#dfn-blank-node).
    BlankNode(String),
}
