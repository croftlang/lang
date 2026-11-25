(function_declaration
  body: (_ "{" (_)* @function.inside "}")) @function.around
(function_declaration) @function.around
(constraint_declaration) @function.around
(policy_declaration body: (block "{" (_)* @function.inside "}")) @function.around

[
  (aggregate_declaration)
  (entity_declaration)
  (value_declaration)
  (enum_declaration)
  (port_declaration)
  (adapter_declaration)
] @class.around

(comment)+ @comment.around
