; The outline panel and symbol search: declarations, and what is inside them.

(context_declaration "context" @context name: (_) @name) @item
(domain_declaration "domain" @context name: (_) @name) @item
(aggregate_declaration "aggregate" @context name: (_) @name) @item
(entity_declaration "entity" @context name: (_) @name) @item
(value_declaration "value" @context name: (_) @name) @item
(enum_declaration "enum" @context name: (_) @name) @item
(enum_variant name: (_) @name) @item
(error_declaration "error" @context name: (_) @name) @item
(event_declaration "event" @context name: (_) @name) @item
(const_declaration "const" @context name: (_) @name) @item
(identity_declaration "identity" @context name: (_) @name) @item
(constraint_declaration "constraint" @context name: (_) @name) @item
(function_declaration kind: _ @context name: (_) @name) @item
(policy_declaration "policy" @context name: (_) @name) @item
(port_declaration "port" @context name: (_) @name) @item
(adapter_declaration "adapter" @context name: (_) @name "for" @context port: (_) @context) @item
(repository_declaration "repository" @context name: (_) @name "for" @context aggregate: (_) @context) @item
(aggregate_declaration (field_declaration name: (_) @name) @item)
(value_declaration (record_body (field_declaration name: (_) @name) @item))
(entity_declaration (record_body (field_declaration name: (_) @name) @item))
