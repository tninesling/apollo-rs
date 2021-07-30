use rowan::GreenNodeBuilder;
use std::fmt;

use crate::{ast::AstNode, ast::Document, Error, SyntaxElement, SyntaxKind};

use super::GraphQLLanguage;

/// An AST generated by the parser.
pub struct SyntaxTree {
    pub(crate) ast: rowan::SyntaxNode<GraphQLLanguage>,
    pub(crate) errors: Vec<crate::Error>,
}

impl SyntaxTree {
    /// Get a reference to the syntax tree's errors.
    pub fn errors(&self) -> &Vec<crate::Error> {
        &self.errors
    }

    /// Return the root typed `Document` node.
    pub fn document(self) -> Document {
        Document { syntax: self.ast }
    }
}

impl fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn print(f: &mut fmt::Formatter<'_>, indent: usize, element: SyntaxElement) -> fmt::Result {
            let kind: SyntaxKind = element.kind();
            print!("{:indent$}", "", indent = indent);
            match element {
                rowan::NodeOrToken::Node(node) => {
                    writeln!(f, "- {:?}@{:?}", kind, node.text_range())?;
                    for child in node.children_with_tokens() {
                        print(f, indent + 2, child)?;
                    }
                    Ok(())
                }

                rowan::NodeOrToken::Token(token) => {
                    writeln!(
                        f,
                        "- {:?}@{:?} {:?}",
                        kind,
                        token.text_range(),
                        token.text()
                    )
                }
            }
        }

        // TODO lrlna: needs a more elegant way of formatting Debug of Errors.
        // Perhaps a separate struct with its own Debug implementation.
        fn print_err(f: &mut fmt::Formatter<'_>, errors: Vec<Error>) -> fmt::Result {
            for err in errors {
                writeln!(f, "- {:?}", err)?;
            }

            write!(f, "")
        }

        print(f, 0, self.ast.clone().into())?;
        print_err(f, self.errors.clone())
    }
}

#[derive(Debug)]
pub(crate) struct SyntaxTreeBuilder {
    builder: GreenNodeBuilder<'static>,
}

impl SyntaxTreeBuilder {
    /// Create a new instance of `SyntaxBuilder`.
    pub(crate) fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
        }
    }

    /// Start new node and make it current.
    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(rowan::SyntaxKind(kind as u16));
    }

    /// Finish current branch and restore previous branch as current.
    pub(crate) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Adds new token to the current branch.
    pub(crate) fn token(&mut self, kind: SyntaxKind, text: &str) {
        self.builder.token(rowan::SyntaxKind(kind as u16), text);
    }

    pub(crate) fn finish(self, errors: Vec<Error>) -> SyntaxTree {
        SyntaxTree {
            ast: rowan::SyntaxNode::new_root(self.builder.finish()),
            // TODO: keep the errors in the builder rather than pass it in here?
            errors,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::AstNode;
    use crate::ast::Definition;
    use crate::Parser;

    #[test]
    fn smoke_directive() {
        let input = "directive @example(isTreat: Boolean, treatKind: String) on FIELD | MUTATION";
        let parser = Parser::new(input);
        let ast = parser.parse();
        let doc = ast.document();

        for def in doc.definitions() {
            if let Definition::DirectiveDefinition(directive) = def {
                println!("{:?}", directive.name().unwrap().ident_token());
            }
        }
    }
}
