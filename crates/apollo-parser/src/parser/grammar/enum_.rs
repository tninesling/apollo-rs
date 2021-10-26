use crate::{
    parser::grammar::{description, directive, name, value},
    Parser, SyntaxKind, TokenKind, S, T,
};

/// See: https://spec.graphql.org/draft/#EnumTypeDefinition
///
/// *EnumTypeDefinition*:
///     Description<sub>opt</sub> **enum** Name Directives<sub>\[Const\] opt</sub> EnumValuesDefinition <sub>opt</sub>
pub(crate) fn enum_type_definition(p: &mut Parser) {
    let _g = p.start_node(SyntaxKind::ENUM_TYPE_DEFINITION);

    if let Some(TokenKind::StringValue) = p.peek() {
        description::description(p);
    }

    if let Some("enum") = p.peek_data().as_deref() {
        p.bump(SyntaxKind::enum_KW);
    }

    match p.peek() {
        Some(TokenKind::Name) => name::name(p),
        _ => p.err("expected a Name"),
    }

    if let Some(T![@]) = p.peek() {
        directive::directives(p);
    }

    if let Some(T!['{']) = p.peek() {
        enum_values_definition(p);
    }
}

/// See: https://spec.graphql.org/draft/#EnumTypeExtension
///
// *EnumTypeExtension*:
///    **extend** **enum** Name Directives<sub>\[Const\] opt</sub> EnumValuesDefinition
///    **extend** **enum** Name Directives<sub>\[Const\]</sub>
pub(crate) fn enum_type_extension(p: &mut Parser) {
    let _g = p.start_node(SyntaxKind::ENUM_TYPE_EXTENSION);
    p.bump(SyntaxKind::extend_KW);
    p.bump(SyntaxKind::enum_KW);

    let mut meets_requirements = false;

    match p.peek() {
        Some(TokenKind::Name) => name::name(p),
        _ => p.err("expected a Name"),
    }

    if let Some(T![@]) = p.peek() {
        meets_requirements = true;
        directive::directives(p);
    }

    if let Some(T!['{']) = p.peek() {
        meets_requirements = true;
        enum_values_definition(p);
    }

    if !meets_requirements {
        p.err("expected Directived or Enum Values Definition");
    }
}

/// See: https://spec.graphql.org/draft/#EnumValuesDefinition
///
/// *EnumValuesDefinition*:
///     **{** EnumValueDefinition<sub>list</sub> **}**
pub(crate) fn enum_values_definition(p: &mut Parser) {
    let _g = p.start_node(SyntaxKind::ENUM_VALUES_DEFINITION);
    p.bump(S!['{']);

    match p.peek() {
        Some(TokenKind::Name | TokenKind::StringValue) => enum_value_definition(p),
        _ => p.err("expected Enum Value Definition"),
    }

    p.expect(T!['}'], S!['}']);
}

/// See: https://spec.graphql.org/draft/#EnumValueDefinition
///
/// *EnumValueDefinition*:
///     Description<sub>opt</sub> EnumValue Directives<sub>\[Const\] opt</sub>
pub(crate) fn enum_value_definition(p: &mut Parser) {
    if let Some(TokenKind::Name | TokenKind::StringValue) = p.peek() {
        let guard = p.start_node(SyntaxKind::ENUM_VALUE_DEFINITION);

        if let Some(TokenKind::StringValue) = p.peek() {
            description::description(p);
        }

        value::enum_value(p);

        if let Some(T![@]) = p.peek() {
            directive::directives(p);
        }
        if p.peek().is_some() {
            guard.finish_node();
            return enum_value_definition(p);
        }
    }

    if let Some(T!['}']) = p.peek() {
        return;
    }
}
