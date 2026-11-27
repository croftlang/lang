; Highlights for Croft. Later patterns win, as in Zed and Neovim, so general
; captures come first and more specific ones after them.

; Names

(identifier) @variable

((identifier) @type
  (#match? @type "^[A-Z]"))

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

((identifier) @variable.special
  (#eq? @variable.special "_"))

(type_identifier) @type

((type_identifier) @type.builtin
  (#match? @type.builtin "^(Int|Float|Decimal|Bool|String|Bytes|Unit|List|Map|Option|Result)$"))

(field_identifier) @property

(field_declaration name: (identifier) @property)
(named_argument name: (identifier) @property)

(parameter name: (identifier) @variable.parameter)
(parameter_constraint name: (identifier) @variable.parameter)
(lambda_expression parameter: (identifier) @variable.parameter)
(binding) @variable.parameter

; Declarations

(function_declaration name: (identifier) @function)
(constraint_declaration name: (identifier) @function)
(policy_declaration name: (type_identifier) @function)
(const_declaration name: (identifier) @constant)
(enum_variant name: (identifier) @constructor)

; Calls, constraints and patterns

(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function))
(native_call function: (identifier) @function)
(issue_expression command: (identifier) @function)

((identifier) @constructor
  (#match? @constructor "^(Some|None|Ok|Err)$"))

(record_expression name: (identifier) @type)
(record_expression name: (field_expression field: (field_identifier) @type))

; In a path, `Room` in `Room.Error` is a type; the last segment of a
; constraint or pattern is the constraint or variant itself.
((path (identifier) @type)
  (#match? @type "^[A-Z]"))
((path (identifier) @constant)
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))
(constraint_use name: (path (identifier) @function .))
(pattern name: (path (identifier) @constructor .))

; Literals

(integer) @number
(float) @number
(boolean) @boolean
(string) @string
(escape_sequence) @string.escape
(comment) @comment

; Keywords

[
  "context"
  "domain"
  "publish"
  "use"
  "from"
  "as"
  "value"
  "entity"
  "aggregate"
  "identity"
  "enum"
  "include"
  "const"
  "constraint"
  "invariant"
  "error"
  "event"
  "command"
  "factory"
  "policy"
  "when"
  "fn"
  "service"
  "translation"
  "port"
  "adapter"
  "repository"
  "for"
  "contexts"
  "calls"
  "let"
  "set"
  "loop"
  "emit"
  "issue"
  "on"
  "if"
  "match"
  "with"
  "where"
  "each"
  "native"
  "or"
] @keyword

(break_expression) @keyword

; Operators and punctuation

[
  "||" "&&" "==" "!=" "<=" ">=" "<<" ">>"
  "+" "-" "*" "/" "%" "|" "^" "&" "!" "~"
  "=" "->" "?"
] @operator

(binary_expression operator: ["<" ">"] @operator)

["(" ")" "{" "}" "[" "]"] @punctuation.bracket

(type_arguments ["<" ">"] @punctuation.bracket)
(type_parameters ["<" ">"] @punctuation.bracket)

["," "." ":"] @punctuation.delimiter
