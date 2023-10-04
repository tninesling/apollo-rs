use apollo_compiler::parse_mixed;
use apollo_compiler::ExecutableDocument;
use apollo_compiler::Schema;

#[test]
fn get_operations() {
    let type_system = r#"
    type Query {
      name: String
    }
    "#;
    let op = r#"{ name }"#;
    let named_op = r#"query getName { name } "#;
    let several_named_op = r#"query getName { name } query getAnotherName { name }"#;
    let noop = r#""#;

    let schema = Schema::parse(type_system, "ts.graphql");
    let doc = ExecutableDocument::parse(&schema, op, "op.graphql");
    assert!(doc.get_operation(None).is_ok());

    let doc = ExecutableDocument::parse(&schema, named_op, "op.graphql");
    assert!(doc.get_operation(Some("getName")).is_ok());
    assert!(doc.get_operation(None).is_ok());

    let doc = ExecutableDocument::parse(&schema, several_named_op, "op.graphql");
    assert!(doc.get_operation(Some("getName")).is_ok());
    assert!(doc.get_operation(None).is_err());

    let doc = ExecutableDocument::parse(&schema, noop, "op.graphql");
    assert!(doc.get_operation(Some("getName")).is_err());
    assert!(doc.get_operation(None).is_err());
}

#[test]
fn is_introspection_operation() {
    let query_input = r#"
        type Query {}
        query TypeIntrospect {
          __type(name: "User") {
            name
            fields {
              name
              type {
                name
              }
            }
          }
          __schema {
            types {
              fields {
                name
              }
            }
          }
        }
    "#;
    let (_, doc) = parse_mixed(query_input, "query.graphql");
    assert!(doc.named_operations["TypeIntrospect"].is_introspection(&doc));
}

#[test]
fn is_not_introspection_operation() {
    let query_input = r#"
        type Query {
            isKagoshimaWagyuInstock: Boolean
        }

        query CheckStock {
          isKagoshimaWagyuInstock

          __schema {
            types {
              fields {
                name
              }
            }
          }
        }
    "#;
    let mutation_input = r#"
        type Mutation {
            buyA5Wagyu(pounds: Int): String
        }

        mutation PurchaseBasket {
            buyA5Wagyu(pounds: 15)
        }
    "#;

    let (_, query_doc) = parse_mixed(query_input, "query.graphql");
    let (_, mutation_doc) = parse_mixed(mutation_input, "mutation.graphql");

    assert!(!query_doc.named_operations["CheckStock"].is_introspection(&query_doc));
    assert!(!mutation_doc.named_operations["PurchaseBasket"].is_introspection(&mutation_doc));
}

#[test]
fn is_introspection_deep() {
    let query_input = r#"
      schema {
        query: Root
      }

      type Root {
        species(id: String): Species
      }

      type Species {
        name: String
      }

      query IntrospectDeepFragments {
        ...onRootTrippy
      }

      fragment onRootTrippy on Root {
         ...onRooten
      }

      fragment onRooten on Root {
        ...onRooten2

        ... on Root {
          __schema {
            types {
              name
            }
          }
        }
      }

      fragment onRooten2 on Root {
         __type(name: "Root") {
          ...onType
        }
        ... on Root {
          __schema {
            directives {
              name
            }
          }
        }
      }

      fragment onType on __Type {
        fields {
          name
        }
      }

      fragment onRooten2_not_intro on Root {
        species(id: "Ewok") {
          name
        }

        ... on Root {
          __schema {
            directives {
              name
            }
          }
        }
     }
    "#;

    let query_input_not_introspect = query_input.replace("...onRooten2", "...onRooten2_not_intro");

    let (_, query_doc) = parse_mixed(query_input, "query.graphql");
    let (_, query_not_introspect_doc) = parse_mixed(query_input_not_introspect, "query2.graphql");

    assert!(query_doc.named_operations["IntrospectDeepFragments"].is_introspection(&query_doc));
    assert!(
        !query_not_introspect_doc.named_operations["IntrospectDeepFragments"]
            .is_introspection(&query_not_introspect_doc)
    );
}

#[test]
fn is_introspection_repeated_fragment() {
    let query_input_indirect = r#"
      type Query {}

      query IntrospectRepeatedIndirectFragment {
        ...A
        ...B
      }

      fragment A on Query { ...C }
      fragment B on Query { ...C }

      fragment C on Query {
        __schema {
          types {
            name
          }
        }
      }
    "#;

    let query_input_direct = r#"
      type Query {}

      query IntrospectRepeatedDirectFragment {
        ...C
        ...C
      }

      fragment C on Query {
        __schema {
          types {
            name
          }
        }
      }
    "#;

    let (_, query_doc_indirect) = parse_mixed(query_input_indirect, "indirect.graphql");
    let (_, query_doc_direct) = parse_mixed(query_input_direct, "direct.graphql");

    assert!(
        query_doc_indirect.named_operations["IntrospectRepeatedIndirectFragment"]
            .is_introspection(&query_doc_indirect)
    );
    assert!(
        query_doc_direct.named_operations["IntrospectRepeatedDirectFragment"]
            .is_introspection(&query_doc_direct)
    );
}
