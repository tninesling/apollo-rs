use super::*;
use crate::name;
use crate::parser::Parser;
use crate::parser::SourceSpan;
use crate::schema::SchemaBuilder;
use crate::validation::DiagnosticList;
use crate::validation::Valid;
use crate::validation::WithErrors;
use crate::ExecutableDocument;
use crate::Schema;
use std::fmt;
use std::hash;
use std::path::Path;
use std::sync::OnceLock;

impl Document {
    /// Create an empty document
    pub fn new() -> Self {
        Self {
            sources: Default::default(),
            definitions: Vec::new(),
        }
    }

    /// Return a new configurable parser
    pub fn parser() -> Parser {
        Parser::default()
    }

    /// Parse `input` with the default configuration
    ///
    /// `path` is the filesystem path (or arbitrary string) used in diagnostics
    /// to identify this source file to users.
    pub fn parse(
        source_text: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, WithErrors<Self>> {
        Self::parser().parse_ast(source_text, path)
    }

    /// Validate as an executable document, as much as possible without a schema
    pub fn validate_standalone_executable(&self) -> Result<(), DiagnosticList> {
        let mut errors = DiagnosticList::new(self.sources.clone());
        let type_system_definitions_are_errors = true;
        let executable = crate::executable::from_ast::document_from_ast(
            None,
            self,
            &mut errors,
            type_system_definitions_are_errors,
        );
        crate::executable::validation::validate_standalone_executable(&mut errors, &executable);
        errors.into_result()
    }

    /// Build a schema with this AST document as its sole input.
    #[allow(clippy::result_large_err)] // Typically not called very often
    pub fn to_schema(&self) -> Result<Schema, WithErrors<Schema>> {
        let mut builder = Schema::builder();
        let executable_definitions_are_errors = true;
        builder.add_ast_document(self, executable_definitions_are_errors);
        builder.build()
    }

    /// Build and validate a schema with this AST document as its sole input.
    #[allow(clippy::result_large_err)] // Typically not called very often
    pub fn to_schema_validate(&self) -> Result<Valid<Schema>, WithErrors<Schema>> {
        let mut builder = Schema::builder();
        let executable_definitions_are_errors = true;
        builder.add_ast_document(self, executable_definitions_are_errors);
        let (mut schema, mut errors) = builder.build_inner();
        crate::schema::validation::validate_schema(&mut errors, &mut schema);
        errors.into_valid_result(schema)
    }

    /// Build an executable document from this AST, with the given schema
    #[allow(clippy::result_large_err)] // Typically not called very often
    pub fn to_executable(
        &self,
        schema: &Valid<Schema>,
    ) -> Result<ExecutableDocument, WithErrors<ExecutableDocument>> {
        let mut errors = DiagnosticList::new(self.sources.clone());
        let document = self.to_executable_inner(schema, &mut errors);
        errors.into_result_with(document)
    }

    /// Build and validate an executable document from this AST, with the given schema
    #[allow(clippy::result_large_err)] // Typically not called very often
    pub fn to_executable_validate(
        &self,
        schema: &Valid<Schema>,
    ) -> Result<Valid<ExecutableDocument>, WithErrors<ExecutableDocument>> {
        let mut errors = DiagnosticList::new(self.sources.clone());
        let document = self.to_executable_inner(schema, &mut errors);
        crate::executable::validation::validate_executable_document(&mut errors, schema, &document);
        errors.into_valid_result(document)
    }

    pub(crate) fn to_executable_inner(
        &self,
        schema: &Valid<Schema>,
        errors: &mut DiagnosticList,
    ) -> ExecutableDocument {
        let type_system_definitions_are_errors = true;
        crate::executable::from_ast::document_from_ast(
            Some(schema),
            self,
            errors,
            type_system_definitions_are_errors,
        )
    }

    /// Build a schema and executable document from this AST containing a mixture
    /// of type system definitions and executable definitions, and validate them.
    /// This is mostly useful for unit tests.
    pub fn to_mixed_validate(
        &self,
    ) -> Result<(Valid<Schema>, Valid<ExecutableDocument>), DiagnosticList> {
        let mut builder = SchemaBuilder::new();
        let executable_definitions_are_errors = false;
        let type_system_definitions_are_errors = false;
        builder.add_ast_document(self, executable_definitions_are_errors);
        let (mut schema, mut errors) = builder.build_inner();
        let executable = crate::executable::from_ast::document_from_ast(
            Some(&schema),
            self,
            &mut errors,
            type_system_definitions_are_errors,
        );
        crate::schema::validation::validate_schema(&mut errors, &mut schema);
        crate::executable::validation::validate_executable_document(
            &mut errors,
            &schema,
            &executable,
        );
        errors
            .into_result()
            .map(|()| (Valid(schema), Valid(executable)))
    }

    serialize_method!();
}

/// `source` is ignored for comparison
impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.definitions == other.definitions
    }
}

impl Eq for Document {}

impl hash::Hash for Document {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.definitions.hash(state);
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Skip two not-useful indentation levels
        for def in &self.definitions {
            def.fmt(f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl Definition {
    /// Returns true if this is an executable definition (operation or fragment).
    pub fn is_executable_definition(&self) -> bool {
        matches!(
            self,
            Self::OperationDefinition(_) | Self::FragmentDefinition(_)
        )
    }

    /// Returns true if this is an extension of another definition.
    pub fn is_extension_definition(&self) -> bool {
        matches!(
            self,
            Self::SchemaExtension(_)
                | Self::ScalarTypeExtension(_)
                | Self::ObjectTypeExtension(_)
                | Self::InterfaceTypeExtension(_)
                | Self::UnionTypeExtension(_)
                | Self::EnumTypeExtension(_)
                | Self::InputObjectTypeExtension(_)
        )
    }

    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Self::OperationDefinition(_) => "an operation definition",
            Self::FragmentDefinition(_) => "a fragment definition",
            Self::DirectiveDefinition(_) => "a directive definition",
            Self::ScalarTypeDefinition(_) => "a scalar type definition",
            Self::ObjectTypeDefinition(_) => "an object type definition",
            Self::InterfaceTypeDefinition(_) => "an interface type definition",
            Self::UnionTypeDefinition(_) => "a union type definition",
            Self::EnumTypeDefinition(_) => "an enum type definition",
            Self::InputObjectTypeDefinition(_) => "an input object type definition",
            Self::SchemaDefinition(_) => "a schema definition",
            Self::SchemaExtension(_) => "a schema extension",
            Self::ScalarTypeExtension(_) => "a scalar type extension",
            Self::ObjectTypeExtension(_) => "an object type extension",
            Self::InterfaceTypeExtension(_) => "an interface type extension",
            Self::UnionTypeExtension(_) => "a union type extension",
            Self::EnumTypeExtension(_) => "an enum type extension",
            Self::InputObjectTypeExtension(_) => "an input object type extension",
        }
    }

    /// If this node was parsed from a source file, returns the file ID and source span
    /// (start and end byte offsets) within that file.
    pub fn location(&self) -> Option<SourceSpan> {
        match self {
            Self::OperationDefinition(def) => def.location(),
            Self::FragmentDefinition(def) => def.location(),
            Self::DirectiveDefinition(def) => def.location(),
            Self::SchemaDefinition(def) => def.location(),
            Self::ScalarTypeDefinition(def) => def.location(),
            Self::ObjectTypeDefinition(def) => def.location(),
            Self::InterfaceTypeDefinition(def) => def.location(),
            Self::UnionTypeDefinition(def) => def.location(),
            Self::EnumTypeDefinition(def) => def.location(),
            Self::InputObjectTypeDefinition(def) => def.location(),
            Self::SchemaExtension(def) => def.location(),
            Self::ScalarTypeExtension(def) => def.location(),
            Self::ObjectTypeExtension(def) => def.location(),
            Self::InterfaceTypeExtension(def) => def.location(),
            Self::UnionTypeExtension(def) => def.location(),
            Self::EnumTypeExtension(def) => def.location(),
            Self::InputObjectTypeExtension(def) => def.location(),
        }
    }

    /// Return the name of this type definition or extension.
    ///
    /// Operations may be anonymous, and schema definitions and extensions never have a name.
    /// In those cases this method returns `None`.
    pub fn name(&self) -> Option<&Name> {
        match self {
            Self::OperationDefinition(def) => def.name.as_ref(),
            Self::FragmentDefinition(def) => Some(&def.name),
            Self::DirectiveDefinition(def) => Some(&def.name),
            Self::SchemaDefinition(_) => None,
            Self::ScalarTypeDefinition(def) => Some(&def.name),
            Self::ObjectTypeDefinition(def) => Some(&def.name),
            Self::InterfaceTypeDefinition(def) => Some(&def.name),
            Self::UnionTypeDefinition(def) => Some(&def.name),
            Self::EnumTypeDefinition(def) => Some(&def.name),
            Self::InputObjectTypeDefinition(def) => Some(&def.name),
            Self::SchemaExtension(_) => None,
            Self::ScalarTypeExtension(def) => Some(&def.name),
            Self::ObjectTypeExtension(def) => Some(&def.name),
            Self::InterfaceTypeExtension(def) => Some(&def.name),
            Self::UnionTypeExtension(def) => Some(&def.name),
            Self::EnumTypeExtension(def) => Some(&def.name),
            Self::InputObjectTypeExtension(def) => Some(&def.name),
        }
    }

    pub fn directives(&self) -> &DirectiveList {
        static EMPTY: DirectiveList = DirectiveList(Vec::new());
        match self {
            Self::DirectiveDefinition(_) => &EMPTY,
            Self::OperationDefinition(def) => &def.directives,
            Self::FragmentDefinition(def) => &def.directives,
            Self::SchemaDefinition(def) => &def.directives,
            Self::ScalarTypeDefinition(def) => &def.directives,
            Self::ObjectTypeDefinition(def) => &def.directives,
            Self::InterfaceTypeDefinition(def) => &def.directives,
            Self::UnionTypeDefinition(def) => &def.directives,
            Self::EnumTypeDefinition(def) => &def.directives,
            Self::InputObjectTypeDefinition(def) => &def.directives,
            Self::SchemaExtension(def) => &def.directives,
            Self::ScalarTypeExtension(def) => &def.directives,
            Self::ObjectTypeExtension(def) => &def.directives,
            Self::InterfaceTypeExtension(def) => &def.directives,
            Self::UnionTypeExtension(def) => &def.directives,
            Self::EnumTypeExtension(def) => &def.directives,
            Self::InputObjectTypeExtension(def) => &def.directives,
        }
    }

    pub fn as_operation_definition(&self) -> Option<&Node<OperationDefinition>> {
        if let Self::OperationDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_fragment_definition(&self) -> Option<&Node<FragmentDefinition>> {
        if let Self::FragmentDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_directive_definition(&self) -> Option<&Node<DirectiveDefinition>> {
        if let Self::DirectiveDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_schema_definition(&self) -> Option<&Node<SchemaDefinition>> {
        if let Self::SchemaDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_scalar_type_definition(&self) -> Option<&Node<ScalarTypeDefinition>> {
        if let Self::ScalarTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_object_type_definition(&self) -> Option<&Node<ObjectTypeDefinition>> {
        if let Self::ObjectTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_interface_type_definition(&self) -> Option<&Node<InterfaceTypeDefinition>> {
        if let Self::InterfaceTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_union_type_definition(&self) -> Option<&Node<UnionTypeDefinition>> {
        if let Self::UnionTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_enum_type_definition(&self) -> Option<&Node<EnumTypeDefinition>> {
        if let Self::EnumTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_input_object_type_definition(&self) -> Option<&Node<InputObjectTypeDefinition>> {
        if let Self::InputObjectTypeDefinition(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_schema_extension(&self) -> Option<&Node<SchemaExtension>> {
        if let Self::SchemaExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_scalar_type_extension(&self) -> Option<&Node<ScalarTypeExtension>> {
        if let Self::ScalarTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_object_type_extension(&self) -> Option<&Node<ObjectTypeExtension>> {
        if let Self::ObjectTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_interface_type_extension(&self) -> Option<&Node<InterfaceTypeExtension>> {
        if let Self::InterfaceTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_union_type_extension(&self) -> Option<&Node<UnionTypeExtension>> {
        if let Self::UnionTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_enum_type_extension(&self) -> Option<&Node<EnumTypeExtension>> {
        if let Self::EnumTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    pub fn as_input_object_type_extension(&self) -> Option<&Node<InputObjectTypeExtension>> {
        if let Self::InputObjectTypeExtension(def) = self {
            Some(def)
        } else {
            None
        }
    }

    serialize_method!();
}

impl fmt::Debug for Definition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Skip the enum variant name as it’s redundant with the struct name in it
        match self {
            Self::OperationDefinition(def) => def.fmt(f),
            Self::FragmentDefinition(def) => def.fmt(f),
            Self::DirectiveDefinition(def) => def.fmt(f),
            Self::SchemaDefinition(def) => def.fmt(f),
            Self::ScalarTypeDefinition(def) => def.fmt(f),
            Self::ObjectTypeDefinition(def) => def.fmt(f),
            Self::InterfaceTypeDefinition(def) => def.fmt(f),
            Self::UnionTypeDefinition(def) => def.fmt(f),
            Self::EnumTypeDefinition(def) => def.fmt(f),
            Self::InputObjectTypeDefinition(def) => def.fmt(f),
            Self::SchemaExtension(def) => def.fmt(f),
            Self::ScalarTypeExtension(def) => def.fmt(f),
            Self::ObjectTypeExtension(def) => def.fmt(f),
            Self::InterfaceTypeExtension(def) => def.fmt(f),
            Self::UnionTypeExtension(def) => def.fmt(f),
            Self::EnumTypeExtension(def) => def.fmt(f),
            Self::InputObjectTypeExtension(def) => def.fmt(f),
        }
    }
}

impl OperationDefinition {
    serialize_method!();
}

impl FragmentDefinition {
    serialize_method!();
}

impl DirectiveDefinition {
    /// Returns the definition of an argument by a given name.
    pub fn argument_by_name(&self, name: &str) -> Option<&Node<InputValueDefinition>> {
        self.arguments.iter().find(|argument| argument.name == name)
    }

    /// Returns the definition of an argument by a given name.
    pub fn argument_by_name_mut(&mut self, name: &str) -> Option<&mut Node<InputValueDefinition>> {
        self.arguments
            .iter_mut()
            .find(|argument| argument.name == name)
    }

    serialize_method!();
}

impl SchemaDefinition {
    serialize_method!();
}

impl ScalarTypeDefinition {
    serialize_method!();
}

impl ObjectTypeDefinition {
    serialize_method!();
}

impl InterfaceTypeDefinition {
    serialize_method!();
}

impl UnionTypeDefinition {
    serialize_method!();
}

impl EnumTypeDefinition {
    serialize_method!();
}

impl InputObjectTypeDefinition {
    serialize_method!();
}

impl SchemaExtension {
    serialize_method!();
}

impl ScalarTypeExtension {
    serialize_method!();
}

impl ObjectTypeExtension {
    serialize_method!();
}

impl InterfaceTypeExtension {
    serialize_method!();
}

impl UnionTypeExtension {
    serialize_method!();
}

impl EnumTypeExtension {
    serialize_method!();
}

impl InputObjectTypeExtension {
    serialize_method!();
}

impl DirectiveList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns an iterator of directives with the given name.
    ///
    /// This method is best for repeatable directives.
    /// See also [`get`][Self::get] for non-repeatable directives.
    pub fn get_all<'def: 'name, 'name>(
        &'def self,
        name: &'name str,
    ) -> impl Iterator<Item = &'def Node<Directive>> + 'name {
        self.0.iter().filter(move |dir| dir.name == name)
    }

    /// Returns an iterator of mutable directives with the given name.
    ///
    /// This method is best for repeatable directives.
    /// See also [`get`][Self::get] for non-repeatable directives.
    pub fn get_all_mut<'def: 'name, 'name>(
        &'def mut self,
        name: &'name str,
    ) -> impl Iterator<Item = &'def mut Node<Directive>> + 'name {
        self.0.iter_mut().filter(move |dir| dir.name == name)
    }

    /// Returns the first directive with the given name, if any.
    ///
    /// This method is best for non-repeatable directives.
    /// See also [`get_all`][Self::get_all] for repeatable directives.
    pub fn get(&self, name: &str) -> Option<&Node<Directive>> {
        self.get_all(name).next()
    }

    /// Returns whether there is a directive with the given name
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Accepts either [`Node<Directive>`] or [`Directive`].
    pub fn push(&mut self, directive: impl Into<Node<Directive>>) {
        self.0.push(directive.into());
    }

    serialize_method!();
}

impl std::fmt::Debug for DirectiveList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for DirectiveList {
    type Target = Vec<Node<Directive>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DirectiveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for DirectiveList {
    type Item = Node<Directive>;

    type IntoIter = std::vec::IntoIter<Node<Directive>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a DirectiveList {
    type Item = &'a Node<Directive>;

    type IntoIter = std::slice::Iter<'a, Node<Directive>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut DirectiveList {
    type Item = &'a mut Node<Directive>;

    type IntoIter = std::slice::IterMut<'a, Node<Directive>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl FromIterator<Node<Directive>> for DirectiveList {
    fn from_iter<T: IntoIterator<Item = Node<Directive>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FromIterator<Directive> for DirectiveList {
    fn from_iter<T: IntoIterator<Item = Directive>>(iter: T) -> Self {
        Self(iter.into_iter().map(Node::new).collect())
    }
}

impl Directive {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            arguments: Vec::new(),
        }
    }

    /// Returns the value of the argument named `name`, accounting for nullability
    /// and for the default value in `schema`’s directive definition.
    pub fn argument_by_name<'doc_or_schema>(
        &'doc_or_schema self,
        name: &str,
        schema: &'doc_or_schema Schema,
    ) -> Result<&'doc_or_schema Node<Value>, ArgumentByNameError> {
        Argument::argument_by_name(&self.arguments, name, || {
            schema
                .directive_definitions
                .get(&self.name)
                .ok_or(ArgumentByNameError::UndefinedDirective)?
                .argument_by_name(name)
                .ok_or(ArgumentByNameError::NoSuchArgument)
        })
    }

    /// Returns the value of the argument named `name`, as specified in the directive application.
    ///
    /// Returns `None` if the directive application does not specify this argument.
    ///
    /// If the directive definition makes this argument nullable or defines a default value,
    /// consider using [`argument_by_name`][Self::argument_by_name] instead.
    pub fn specified_argument_by_name(&self, name: &str) -> Option<&Node<Value>> {
        Argument::specified_argument_by_name(&self.arguments, name)
    }

    /// Returns the value of the argument named `name`, as specified in the directive application.
    /// If there are other [`Node`] pointers to the same argument, this method will clone the
    /// argument using [`Node::make_mut`].
    ///
    /// Returns `None` if the directive application does not specify this argument.
    ///
    /// If the directive definition makes this argument nullable or defines a default value,
    /// consider using [`argument_by_name`][Self::argument_by_name] instead.
    pub fn specified_argument_by_name_mut(&mut self, name: &str) -> Option<&mut Node<Value>> {
        Argument::specified_argument_by_name_mut(&mut self.arguments, name)
    }

    serialize_method!();
}

impl Argument {
    pub(crate) fn argument_by_name<'doc_or_def>(
        arguments: &'doc_or_def [Node<Self>],
        name: &str,
        def: impl FnOnce() -> Result<&'doc_or_def Node<InputValueDefinition>, ArgumentByNameError>,
    ) -> Result<&'doc_or_def Node<Value>, ArgumentByNameError> {
        if let Some(value) = Self::specified_argument_by_name(arguments, name) {
            Ok(value)
        } else {
            let argument_def = def()?;
            if let Some(value) = &argument_def.default_value {
                Ok(value)
            } else if argument_def.ty.is_non_null() {
                Err(ArgumentByNameError::RequiredArgumentNotSpecified)
            } else {
                Ok(Value::static_null())
            }
        }
    }

    pub(crate) fn specified_argument_by_name<'doc>(
        arguments: &'doc [Node<Self>],
        name: &str,
    ) -> Option<&'doc Node<Value>> {
        arguments
            .iter()
            .find_map(|arg| (arg.name == name).then_some(&arg.value))
    }

    pub(crate) fn specified_argument_by_name_mut<'doc>(
        arguments: &'doc mut [Node<Self>],
        name: &str,
    ) -> Option<&'doc mut Node<Value>> {
        arguments
            .iter_mut()
            .find_map(|arg| (arg.name == name).then_some(&mut arg.make_mut().value))
    }
}

impl OperationType {
    /// Get the name of this operation type as it would appear in GraphQL source code.
    pub fn name(self) -> &'static str {
        match self {
            OperationType::Query => "query",
            OperationType::Mutation => "mutation",
            OperationType::Subscription => "subscription",
        }
    }

    /// Get the default name of the object type for this operation type
    pub const fn default_type_name(self) -> NamedType {
        match self {
            OperationType::Query => name!("Query"),
            OperationType::Mutation => name!("Mutation"),
            OperationType::Subscription => name!("Subscription"),
        }
    }

    pub fn is_query(&self) -> bool {
        matches!(self, Self::Query)
    }

    pub fn is_mutation(&self) -> bool {
        matches!(self, Self::Mutation)
    }

    pub fn is_subscription(&self) -> bool {
        matches!(self, Self::Subscription)
    }

    serialize_method!();
}

impl DirectiveLocation {
    /// Get the name of this directive location as it would appear in GraphQL source code.
    pub fn name(self) -> &'static str {
        match self {
            DirectiveLocation::Query => "QUERY",
            DirectiveLocation::Mutation => "MUTATION",
            DirectiveLocation::Subscription => "SUBSCRIPTION",
            DirectiveLocation::Field => "FIELD",
            DirectiveLocation::FragmentDefinition => "FRAGMENT_DEFINITION",
            DirectiveLocation::FragmentSpread => "FRAGMENT_SPREAD",
            DirectiveLocation::InlineFragment => "INLINE_FRAGMENT",
            DirectiveLocation::VariableDefinition => "VARIABLE_DEFINITION",
            DirectiveLocation::Schema => "SCHEMA",
            DirectiveLocation::Scalar => "SCALAR",
            DirectiveLocation::Object => "OBJECT",
            DirectiveLocation::FieldDefinition => "FIELD_DEFINITION",
            DirectiveLocation::ArgumentDefinition => "ARGUMENT_DEFINITION",
            DirectiveLocation::Interface => "INTERFACE",
            DirectiveLocation::Union => "UNION",
            DirectiveLocation::Enum => "ENUM",
            DirectiveLocation::EnumValue => "ENUM_VALUE",
            DirectiveLocation::InputObject => "INPUT_OBJECT",
            DirectiveLocation::InputFieldDefinition => "INPUT_FIELD_DEFINITION",
        }
    }
}

impl fmt::Debug for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name().fmt(f)
    }
}

impl From<OperationType> for DirectiveLocation {
    fn from(ty: OperationType) -> Self {
        match ty {
            OperationType::Query => DirectiveLocation::Query,
            OperationType::Mutation => DirectiveLocation::Mutation,
            OperationType::Subscription => DirectiveLocation::Subscription,
        }
    }
}

impl VariableDefinition {
    serialize_method!();
}

/// Create a static [`Type`] with GraphQL-like syntax
///
/// ```
/// use apollo_compiler::ty;
///
/// assert_eq!(ty!(Obj).to_string(), "Obj");
/// assert_eq!(ty!(Obj!).to_string(), "Obj!");
/// assert_eq!(ty!([Obj]).to_string(), "[Obj]");
/// assert_eq!(ty!([Obj]!).to_string(), "[Obj]!");
/// assert_eq!(ty!([[[Obj ] !]]!).to_string(), "[[[Obj]!]]!");
/// ```
#[macro_export]
macro_rules! ty {
    ($name: ident) => {
        $crate::ast::Type::Named($crate::name!($name))
    };
    ($name: ident !) => {
        $crate::ast::Type::NonNullNamed($crate::name!($name))
    };
    ([ $($tt: tt)+ ]) => {
        $crate::ast::Type::List(::std::boxed::Box::new($crate::ty!( $($tt)+ )))
    };
    ([ $($tt: tt)+ ]!) => {
        $crate::ast::Type::NonNullList(::std::boxed::Box::new($crate::ty!( $($tt)+ )))
    };
}

impl Type {
    /// Returns this type made non-null, if it isn’t already.
    pub fn non_null(self) -> Self {
        match self {
            Type::Named(name) => Type::NonNullNamed(name),
            Type::List(inner) => Type::NonNullList(inner),
            Type::NonNullNamed(_) => self,
            Type::NonNullList(_) => self,
        }
    }

    /// Returns this type made nullable, if it isn’t already.
    pub fn nullable(self) -> Self {
        match self {
            Type::Named(_) => self,
            Type::List(_) => self,
            Type::NonNullNamed(name) => Type::Named(name),
            Type::NonNullList(inner) => Type::List(inner),
        }
    }

    /// Returns a list type whose items are this type.
    pub fn list(self) -> Self {
        Type::List(Box::new(self))
    }

    /// If the type is a list type (nullable or not), returns the inner item type.
    ///
    /// Otherwise returns `self` unchanged.
    ///
    /// # Example
    /// ```
    /// use apollo_compiler::ty;
    /// // Returns the inner type of the list.
    /// assert_eq!(ty!([Foo!]).item_type(), &ty!(Foo!));
    /// // Not a list: returns the input.
    /// assert_eq!(ty!(Foo!).item_type(), &ty!(Foo!));
    /// ```
    pub fn item_type(&self) -> &Self {
        match self {
            Type::List(inner) | Type::NonNullList(inner) => inner,
            ty => ty,
        }
    }

    /// Returns the inner named type, after unwrapping any non-null or list markers.
    pub fn inner_named_type(&self) -> &NamedType {
        match self {
            Type::Named(name) | Type::NonNullNamed(name) => name,
            Type::List(inner) | Type::NonNullList(inner) => inner.inner_named_type(),
        }
    }

    /// Returns whether this type is non-null
    pub fn is_non_null(&self) -> bool {
        matches!(self, Type::NonNullNamed(_) | Type::NonNullList(_))
    }

    /// Returns whether this type is a list type (nullable or not)
    pub fn is_list(&self) -> bool {
        matches!(self, Type::List(_) | Type::NonNullList(_))
    }

    /// Returns whether this type is a named type (nullable or not), as opposed to a list type.
    pub fn is_named(&self) -> bool {
        matches!(self, Type::Named(_) | Type::NonNullNamed(_))
    }

    /// Can a value of this type be used when the `target` type is expected?
    ///
    /// Implementation of spec function
    /// [_AreTypesCompatible()_](https://spec.graphql.org/draft/#AreTypesCompatible()).
    pub fn is_assignable_to(&self, target: &Self) -> bool {
        match (target, self) {
            // Can't assign a nullable type to a non-nullable type.
            (Type::NonNullNamed(_) | Type::NonNullList(_), Type::Named(_) | Type::List(_)) => false,
            // Can't assign a list type to a non-list type.
            (Type::Named(_) | Type::NonNullNamed(_), Type::List(_) | Type::NonNullList(_)) => false,
            // Can't assign a non-list type to a list type.
            (Type::List(_) | Type::NonNullList(_), Type::Named(_) | Type::NonNullNamed(_)) => false,
            // Non-null named types can be assigned if they are the same.
            (Type::NonNullNamed(left), Type::NonNullNamed(right)) => left == right,
            // Non-null list types can be assigned if their inner types are compatible.
            (Type::NonNullList(left), Type::NonNullList(right)) => right.is_assignable_to(left),
            // Both nullable and non-nullable named types can be assigned to a nullable type of the
            // same name.
            (Type::Named(left), Type::Named(right) | Type::NonNullNamed(right)) => left == right,
            // Nullable and non-nullable lists can be assigned to a matching nullable list type.
            (Type::List(left), Type::List(right) | Type::NonNullList(right)) => {
                right.is_assignable_to(left)
            }
        }
    }

    /// Parse the given source text as a reference to a type.
    ///
    /// `path` is the filesystem path (or arbitrary string) used in diagnostics
    /// to identify this source file to users.
    ///
    /// Create a [`Parser`] to use different parser configuration.
    pub fn parse(
        source_text: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, DiagnosticList> {
        Parser::new().parse_type(source_text, path)
    }

    serialize_method!();
}

impl FieldDefinition {
    /// Returns the definition of an argument by a given name.
    pub fn argument_by_name(&self, name: &str) -> Option<&Node<InputValueDefinition>> {
        self.arguments.iter().find(|argument| argument.name == name)
    }

    serialize_method!();
}

impl InputValueDefinition {
    /// Returns true if usage sites are required to provide a value for this input value.
    ///
    /// An input value is required when:
    /// - its type is non-null, and
    /// - it does not have a default value
    pub fn is_required(&self) -> bool {
        self.ty.is_non_null() && self.default_value.is_none()
    }

    serialize_method!();
}

impl EnumValueDefinition {
    serialize_method!();
}

impl Selection {
    /// If this node was parsed from a source file, returns the file ID and source span
    /// (start and end byte offsets) within that file.
    pub fn location(&self) -> Option<SourceSpan> {
        match self {
            Self::Field(field) => field.location(),
            Self::FragmentSpread(fragment) => fragment.location(),
            Self::InlineFragment(fragment) => fragment.location(),
        }
    }

    pub fn as_field(&self) -> Option<&Node<Field>> {
        if let Self::Field(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn as_fragment_spread(&self) -> Option<&Node<FragmentSpread>> {
        if let Self::FragmentSpread(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn as_inline_fragment(&self) -> Option<&Node<InlineFragment>> {
        if let Self::InlineFragment(x) = self {
            Some(x)
        } else {
            None
        }
    }

    serialize_method!();
}

impl Field {
    /// Get the name that will be used for this field selection in [response `data`].
    ///
    /// For example, in this operation, the response name is `sourceField`:
    /// ```graphql
    /// query GetField { sourceField }
    /// ```
    ///
    /// But in this operation that uses an alias, the response name is `responseField`:
    /// ```graphql
    /// query GetField { responseField: sourceField }
    /// ```
    ///
    /// [response `data`]: https://spec.graphql.org/draft/#sec-Response-Format
    pub fn response_name(&self) -> &Name {
        self.alias.as_ref().unwrap_or(&self.name)
    }

    serialize_method!();
}

impl FragmentSpread {
    serialize_method!();
}

impl InlineFragment {
    serialize_method!();
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_enum(&self) -> Option<&Name> {
        if let Value::Enum(name) = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn as_variable(&self) -> Option<&Name> {
        if let Value::Variable(name) = self {
            Some(name)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Convert a [`FloatValue`] **_or [`IntValue`]_** to floating point representation.
    ///
    /// Returns `None` if the value is of a different kind, or if the conversion overflows.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Value::Float(value) => value.try_to_f64().ok(),
            Value::Int(value) => value.try_to_f64().ok(),
            _ => None,
        }
    }

    pub fn to_i32(&self) -> Option<i32> {
        if let Value::Int(value) = self {
            value.try_to_i32().ok()
        } else {
            None
        }
    }

    pub fn to_bool(&self) -> Option<bool> {
        if let Value::Boolean(value) = *self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_list(&self) -> Option<&[Node<Value>]> {
        if let Value::List(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<&[(Name, Node<Value>)]> {
        if let Value::Object(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Enum(_) => "an enum",
            Value::Variable(_) => "a variable",
            Value::String(_) => "a string",
            Value::Float(_) => "a float",
            Value::Int(_) => "an integer",
            Value::Boolean(_) => "a boolean",
            Value::List(_) => "a list",
            Value::Object(_) => "an input object",
        }
    }

    pub(crate) fn static_null() -> &'static Node<Self> {
        static NULL: OnceLock<Node<Value>> = OnceLock::new();
        NULL.get_or_init(|| Value::Null.into())
    }

    serialize_method!();
}

impl IntValue {
    /// Constructs from a string matching the [`IntValue`
    /// grammar specification](https://spec.graphql.org/October2021/#IntValue)
    ///
    /// To convert an `i32`, use `from` or `into` instead.
    pub fn new_parsed(text: &str) -> Self {
        debug_assert!(IntValue::valid_syntax(text), "{text:?}");
        Self(text.into())
    }

    fn valid_syntax(text: &str) -> bool {
        match text.strip_prefix('-').unwrap_or(text).as_bytes() {
            [b'0'..=b'9'] => true,
            [b'1'..=b'9', rest @ ..] => rest.iter().all(|b| b.is_ascii_digit()),
            _ => false,
        }
    }

    /// Returns the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts to `i32`, returning an error on overflow
    ///
    /// Note: parsing is expected to succeed with a correctly-constructed `IntValue`,
    /// leaving overflow as the only error case.
    pub fn try_to_i32(&self) -> Result<i32, std::num::ParseIntError> {
        self.0.parse()
    }

    /// Converts to a finite `f64`, returning an error on overflow to infinity
    ///
    /// An `IntValue` signals integer syntax was used, but is also valid in contexts
    /// where a `Float` is expected.
    ///
    /// Note: parsing is expected to succeed with a correctly-constructed `IntValue`,
    /// leaving overflow as the only error case.
    pub fn try_to_f64(&self) -> Result<f64, FloatOverflowError> {
        try_to_f64(&self.0)
    }
}

impl FloatValue {
    /// Constructs from a string matching the [`FloatValue`
    /// grammar specification](https://spec.graphql.org/October2021/#IntValue)
    ///
    /// To convert an `f64`, use `from` or `into` instead.
    pub fn new_parsed(text: &str) -> Self {
        debug_assert!(FloatValue::valid_syntax(text), "{text:?}");
        Self(text.into())
    }

    fn valid_syntax(text: &str) -> bool {
        if let Some((mantissa, exponent)) = text.split_once(['e', 'E']) {
            let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            if !exponent.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            if let Some((int, fract)) = mantissa.split_once('.') {
                Self::valid_fractional_syntax(int, fract)
            } else {
                IntValue::valid_syntax(mantissa)
            }
        } else {
            text.split_once('.')
                .is_some_and(|(int, fract)| Self::valid_fractional_syntax(int, fract))
        }
    }

    fn valid_fractional_syntax(integer: &str, fractional: &str) -> bool {
        IntValue::valid_syntax(integer)
            && !fractional.is_empty()
            && fractional.bytes().all(|b| b.is_ascii_digit())
    }

    /// Returns the string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts to a finite `f64`, returning an error on overflow to infinity
    ///
    /// Note: parsing is expected to succeed with a correctly-constructed `FloatValue`,
    /// leaving overflow as the only error case.
    pub fn try_to_f64(&self) -> Result<f64, FloatOverflowError> {
        try_to_f64(&self.0)
    }
}

fn try_to_f64(text: &str) -> Result<f64, FloatOverflowError> {
    let parsed = text.parse::<f64>();
    debug_assert!(parsed.is_ok(), "{}", parsed.unwrap_err());
    let Ok(float) = parsed else {
        return Err(FloatOverflowError {});
    };
    debug_assert!(!float.is_nan());
    if float.is_finite() {
        Ok(float)
    } else {
        Err(FloatOverflowError {})
    }
}

impl From<i32> for IntValue {
    fn from(value: i32) -> Self {
        let text = value.to_string();
        debug_assert!(IntValue::valid_syntax(&text), "{text:?}");
        Self(text)
    }
}

impl From<f64> for FloatValue {
    fn from(value: f64) -> Self {
        let mut text = value.to_string();
        if !text.contains('.') {
            text.push_str(".0")
        }
        debug_assert!(FloatValue::valid_syntax(&text), "{text:?}");
        Self(text)
    }
}

impl fmt::Display for IntValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for FloatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for IntValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for FloatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<'de> serde::Deserialize<'de> for IntValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const EXPECTING: &str = "a string in GraphQL IntValue syntax";
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = IntValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(EXPECTING)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if IntValue::valid_syntax(v) {
                    Ok(IntValue(v.to_owned()))
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(v), &EXPECTING))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if IntValue::valid_syntax(&v) {
                    Ok(IntValue(v))
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(&v), &EXPECTING))
                }
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

impl<'de> serde::Deserialize<'de> for FloatValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const EXPECTING: &str = "a string in GraphQL FloatValue syntax";
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = FloatValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(EXPECTING)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if FloatValue::valid_syntax(v) {
                    Ok(FloatValue(v.to_owned()))
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(v), &EXPECTING))
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if FloatValue::valid_syntax(&v) {
                    Ok(FloatValue(v))
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(&v), &EXPECTING))
                }
            }
        }
        deserializer.deserialize_string(Visitor)
    }
}

impl serde::Serialize for IntValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl serde::Serialize for FloatValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl fmt::Display for FloatOverflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("value magnitude too large to be converted to `f64`")
    }
}

impl fmt::Debug for FloatOverflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<Node<OperationDefinition>> for Definition {
    fn from(def: Node<OperationDefinition>) -> Self {
        Self::OperationDefinition(def)
    }
}

impl From<Node<FragmentDefinition>> for Definition {
    fn from(def: Node<FragmentDefinition>) -> Self {
        Self::FragmentDefinition(def)
    }
}

impl From<Node<DirectiveDefinition>> for Definition {
    fn from(def: Node<DirectiveDefinition>) -> Self {
        Self::DirectiveDefinition(def)
    }
}

impl From<Node<SchemaDefinition>> for Definition {
    fn from(def: Node<SchemaDefinition>) -> Self {
        Self::SchemaDefinition(def)
    }
}

impl From<Node<ScalarTypeDefinition>> for Definition {
    fn from(def: Node<ScalarTypeDefinition>) -> Self {
        Self::ScalarTypeDefinition(def)
    }
}

impl From<Node<ObjectTypeDefinition>> for Definition {
    fn from(def: Node<ObjectTypeDefinition>) -> Self {
        Self::ObjectTypeDefinition(def)
    }
}

impl From<Node<InterfaceTypeDefinition>> for Definition {
    fn from(def: Node<InterfaceTypeDefinition>) -> Self {
        Self::InterfaceTypeDefinition(def)
    }
}

impl From<Node<UnionTypeDefinition>> for Definition {
    fn from(def: Node<UnionTypeDefinition>) -> Self {
        Self::UnionTypeDefinition(def)
    }
}

impl From<Node<EnumTypeDefinition>> for Definition {
    fn from(def: Node<EnumTypeDefinition>) -> Self {
        Self::EnumTypeDefinition(def)
    }
}

impl From<Node<InputObjectTypeDefinition>> for Definition {
    fn from(def: Node<InputObjectTypeDefinition>) -> Self {
        Self::InputObjectTypeDefinition(def)
    }
}

impl From<Node<SchemaExtension>> for Definition {
    fn from(def: Node<SchemaExtension>) -> Self {
        Self::SchemaExtension(def)
    }
}

impl From<Node<ScalarTypeExtension>> for Definition {
    fn from(def: Node<ScalarTypeExtension>) -> Self {
        Self::ScalarTypeExtension(def)
    }
}

impl From<Node<ObjectTypeExtension>> for Definition {
    fn from(def: Node<ObjectTypeExtension>) -> Self {
        Self::ObjectTypeExtension(def)
    }
}

impl From<Node<InterfaceTypeExtension>> for Definition {
    fn from(def: Node<InterfaceTypeExtension>) -> Self {
        Self::InterfaceTypeExtension(def)
    }
}

impl From<Node<UnionTypeExtension>> for Definition {
    fn from(def: Node<UnionTypeExtension>) -> Self {
        Self::UnionTypeExtension(def)
    }
}

impl From<Node<EnumTypeExtension>> for Definition {
    fn from(def: Node<EnumTypeExtension>) -> Self {
        Self::EnumTypeExtension(def)
    }
}

impl From<Node<InputObjectTypeExtension>> for Definition {
    fn from(def: Node<InputObjectTypeExtension>) -> Self {
        Self::InputObjectTypeExtension(def)
    }
}

impl From<OperationDefinition> for Definition {
    fn from(def: OperationDefinition) -> Self {
        Self::OperationDefinition(Node::new(def))
    }
}

impl From<FragmentDefinition> for Definition {
    fn from(def: FragmentDefinition) -> Self {
        Self::FragmentDefinition(Node::new(def))
    }
}

impl From<DirectiveDefinition> for Definition {
    fn from(def: DirectiveDefinition) -> Self {
        Self::DirectiveDefinition(Node::new(def))
    }
}

impl From<SchemaDefinition> for Definition {
    fn from(def: SchemaDefinition) -> Self {
        Self::SchemaDefinition(Node::new(def))
    }
}

impl From<ScalarTypeDefinition> for Definition {
    fn from(def: ScalarTypeDefinition) -> Self {
        Self::ScalarTypeDefinition(Node::new(def))
    }
}

impl From<ObjectTypeDefinition> for Definition {
    fn from(def: ObjectTypeDefinition) -> Self {
        Self::ObjectTypeDefinition(Node::new(def))
    }
}

impl From<InterfaceTypeDefinition> for Definition {
    fn from(def: InterfaceTypeDefinition) -> Self {
        Self::InterfaceTypeDefinition(Node::new(def))
    }
}

impl From<UnionTypeDefinition> for Definition {
    fn from(def: UnionTypeDefinition) -> Self {
        Self::UnionTypeDefinition(Node::new(def))
    }
}

impl From<EnumTypeDefinition> for Definition {
    fn from(def: EnumTypeDefinition) -> Self {
        Self::EnumTypeDefinition(Node::new(def))
    }
}

impl From<InputObjectTypeDefinition> for Definition {
    fn from(def: InputObjectTypeDefinition) -> Self {
        Self::InputObjectTypeDefinition(Node::new(def))
    }
}

impl From<SchemaExtension> for Definition {
    fn from(def: SchemaExtension) -> Self {
        Self::SchemaExtension(Node::new(def))
    }
}

impl From<ScalarTypeExtension> for Definition {
    fn from(def: ScalarTypeExtension) -> Self {
        Self::ScalarTypeExtension(Node::new(def))
    }
}

impl From<ObjectTypeExtension> for Definition {
    fn from(def: ObjectTypeExtension) -> Self {
        Self::ObjectTypeExtension(Node::new(def))
    }
}

impl From<InterfaceTypeExtension> for Definition {
    fn from(def: InterfaceTypeExtension) -> Self {
        Self::InterfaceTypeExtension(Node::new(def))
    }
}

impl From<UnionTypeExtension> for Definition {
    fn from(def: UnionTypeExtension) -> Self {
        Self::UnionTypeExtension(Node::new(def))
    }
}

impl From<EnumTypeExtension> for Definition {
    fn from(def: EnumTypeExtension) -> Self {
        Self::EnumTypeExtension(Node::new(def))
    }
}

impl From<InputObjectTypeExtension> for Definition {
    fn from(def: InputObjectTypeExtension) -> Self {
        Self::InputObjectTypeExtension(Node::new(def))
    }
}

/// The Rust unit value a.k.a empty tuple converts to [`Value::Null`].
impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Null
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Float(value.into())
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Int(value.into())
    }
}

impl From<&'_ str> for Value {
    fn from(value: &'_ str) -> Self {
        Value::String(value.into())
    }
}

impl From<&'_ String> for Value {
    fn from(value: &'_ String) -> Self {
        Value::String(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

/// The Rust unit value a.k.a empty tuple converts to [`Value::Null`].
impl From<()> for Node<Value> {
    fn from(value: ()) -> Self {
        Node::new(value.into())
    }
}

impl From<f64> for Node<Value> {
    fn from(value: f64) -> Self {
        Node::new(value.into())
    }
}

impl From<i32> for Node<Value> {
    fn from(value: i32) -> Self {
        Node::new(value.into())
    }
}

impl From<&'_ str> for Node<Value> {
    fn from(value: &'_ str) -> Self {
        Node::new(value.into())
    }
}

impl From<&'_ String> for Node<Value> {
    fn from(value: &'_ String) -> Self {
        Node::new(value.into())
    }
}

impl From<String> for Node<Value> {
    fn from(value: String) -> Self {
        Node::new(value.into())
    }
}

impl From<bool> for Node<Value> {
    fn from(value: bool) -> Self {
        Node::new(value.into())
    }
}

impl<N: Into<Name>, V: Into<Node<Value>>> From<(N, V)> for Node<Argument> {
    fn from((name, value): (N, V)) -> Self {
        Node::new(Argument {
            name: name.into(),
            value: value.into(),
        })
    }
}
