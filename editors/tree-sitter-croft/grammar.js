/**
 * @file Tree-sitter grammar for the Croft domain language.
 *
 * It follows the parser in crates/croft_syntax/src/parser. Two things there
 * depend on line breaks, which tree-sitter treats as plain whitespace:
 *
 * - The compiler ends an expression at a new line, except before `.` and
 *   `?`. Here a binary operator or `(` at the start of a line carries the
 *   expression on. Code written that way is rare, and the compiler reads it
 *   as two statements.
 * - The compiler reads `{` after a name in an `if` or `match` condition as
 *   the start of the block, never a record. Here both readings are tried and
 *   the one that parses is kept.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

// The same order as `binary_operator` in parser/expression.rs. Bit operators
// bind tighter than comparisons, unlike in C.
const PREC = {
  lambda: -1,
  or_else: 0,
  or: 1,
  and: 2,
  equality: 3,
  comparison: 4,
  bit_or: 5,
  bit_xor: 6,
  bit_and: 7,
  shift: 8,
  additive: 9,
  multiplicative: 10,
  unary: 11,
  try: 12,
  issue: 13,
  postfix: 14,
};

// crates/croft_syntax/src/keyword.rs. A field name after `.` or in a record can
// be any of these: `score.value`, `Score { value: 3 }`.
const KEYWORDS = [
  'context', 'domain', 'publish', 'use', 'value', 'entity', 'aggregate',
  'identity', 'enum', 'const', 'constraint', 'invariant', 'error', 'event',
  'command', 'factory', 'policy', 'fn', 'service', 'port', 'adapter', 'repository',
  'translation', 'let', 'set', 'loop', 'break', 'emit', 'issue', 'if',
  'match', 'with', 'where', 'each', 'calls', 'native', 'or', 'true', 'false',
];

module.exports = grammar({
  name: 'croft',

  extras: $ => [/\s/, $.comment],

  word: $ => $.identifier,

  conflicts: $ => [
    // `if x == Foo {`: a record, or the end of the condition.
    [$._expression, $.record_expression],
    // `where Range(0, 9), next: Int`: another constraint, or the next field.
    [$.where_clause],
  ],

  rules: {
    source_file: $ => repeat(choice($.domain_declaration, $.context_declaration)),

    comment: _ => token(seq('//', /[^\n]*/)),

    // Files and contexts.

    // `context Dungeon { ... }`, or `context Dungeon` as a header line that
    // claims the rest of the file. A header can end with an `aggregate` header.
    context_declaration: $ => choice(
      seq('context', field('name', $._type_name), field('body', $.declaration_list)),
      prec.right(seq(
        'context',
        field('name', $._type_name),
        repeat($._context_item),
        optional(alias($._aggregate_header, $.aggregate_declaration)),
      )),
    ),

    declaration_list: $ => seq('{', repeat($._context_item), '}'),

    _context_item: $ => choice(
      $.publish_declaration,
      $.use_declaration,
      $.value_declaration,
      $.enum_declaration,
      $.error_declaration,
      $.const_declaration,
      $.identity_declaration,
      $.constraint_declaration,
      $.aggregate_declaration,
      $.function_declaration,
      $.policy_declaration,
      $.port_declaration,
      $.adapter_declaration,
      $.repository_declaration,
    ),

    publish_declaration: $ => seq('publish', $.path_list),

    // `use { EnemyId, Room.EnemyDefeated } from Dungeon`
    use_declaration: $ => seq('use', $.path_list, 'from', field('from', $.path)),

    path_list: $ => seq('{', repeat(seq($.path, optional(','))), '}'),

    path: $ => prec.right(seq($._name, repeat(seq('.', $._name)))),

    // `domain Game { contexts { ... } use { ... } from X  calls Screen } = { ... }`
    domain_declaration: $ => prec.right(seq(
      'domain',
      field('name', $._type_name),
      '{',
      repeat(choice($.context_map, $.use_declaration, $.calls_clause)),
      '}',
      '=',
      field('body', $._expression),
    )),

    // `contexts { Dungeon -> Scoring  Dungeon -> Platform }`, or one context on its own: `contexts { Shop }`
    context_map: $ => seq('contexts', '{', repeat(seq($.context_relation, optional(','))), '}'),

    context_relation: $ => seq(
      typeName($),
      repeat(seq('->', typeName($))),
    ),

    // Data.

    // `value Position { x: Int  y: Int }` or `value Score: Int where Range(0, 999)`
    value_declaration: $ => seq(
      'value',
      field('name', $._type_name),
      optional(field('type_parameters', $.type_parameters)),
      choice(
        seq(':', field('type', $.type), optional(field('constraints', $.where_clause))),
        field('body', $.record_body),
      ),
    ),

    entity_declaration: $ => seq('entity', field('name', $._type_name), field('body', $.record_body)),

    // Fields and invariants, separated by commas or just spaces.
    record_body: $ => seq('{', repeat(seq(choice($.field_declaration, $.invariant), optional(','))), '}'),

    // `id: RoomId` or `hit_points: Int where Range(0, 100)`
    field_declaration: $ => prec.right(seq(
      field('name', $._name),
      ':',
      field('type', $.type),
      optional(field('constraints', $.where_clause)),
      optional(seq('=', field('value', $._expression))),
    )),

    field_list: $ => seq('{', repeat(seq($.field_declaration, optional(','))), '}'),

    enum_declaration: $ => seq(
      'enum',
      field('name', $._type_name),
      '{',
      repeat(seq(choice($.enum_include, $.enum_variant), optional(','))),
      '}',
    ),

    enum_include: $ => seq('include', $.path),

    // `Coins(count: Int where Range(1, 99))` or `None`
    enum_variant: $ => seq(
      field('name', $.identifier),
      optional(seq('(', repeat(seq($.field_declaration, optional(','))), ')')),
    ),

    error_declaration: $ => seq('error', field('name', $._type_name), optional(field('fields', $.field_list))),

    event_declaration: $ => seq('event', field('name', $._type_name), optional(field('fields', $.field_list))),

    const_declaration: $ => prec.right(seq(
      'const',
      field('name', $.identifier),
      ':',
      field('type', $.type),
      '=',
      field('value', $._expression),
    )),

    // `identity RoomId for Room: Int where Range(1, 20)`
    identity_declaration: $ => seq(
      'identity',
      field('name', $._type_name),
      'for',
      field('owner', $.path),
      ':',
      field('type', $.type),
      optional(field('constraints', $.where_clause)),
    ),

    // Aggregates.

    aggregate_declaration: $ => seq(
      'aggregate',
      field('name', $._type_name),
      '{',
      repeat($._aggregate_member),
      '}',
    ),

    // `aggregate Room` after a context header: the rest of the file is its members.
    _aggregate_header: $ => prec.right(seq('aggregate', field('name', $._type_name), repeat($._aggregate_member))),

    _aggregate_member: $ => choice(
      $.entity_declaration,
      $.invariant,
      $.error_declaration,
      $.event_declaration,
      $.function_declaration,
      $.field_declaration,
    ),

    // `invariant ExitClear or ExitBlocked`
    invariant: $ => prec.right(seq(
      'invariant',
      field('constraint', $.constraint_use),
      optional(seq('or', field('error', $._expression))),
    )),

    // Constraints.

    // `where Length(0, MAX_ENEMIES), each Alive`
    where_clause: $ => seq('where', $.constraint_use, repeat(seq(',', $.constraint_use))),

    // `Range(1, 20)`, `Alive` or `each Alive`
    constraint_use: $ => prec.right(seq(
      optional('each'),
      field('name', $.path),
      optional(field('arguments', $.arguments)),
    )),

    // `constraint Alive(enemy: Room.Enemy) = enemy.hit_points > 0`
    constraint_declaration: $ => prec.right(seq(
      'constraint',
      field('name', $.identifier),
      field('parameters', $.parameters),
      '=',
      field('body', $._expression),
    )),

    // Functions and behaviour.

    // Anything with a signature. Only the name is required.
    function_declaration: $ => prec.right(seq(
      field('kind', choice('fn', 'service', 'translation', 'command', 'factory')),
      field('name', $._name),
      optional(field('type_parameters', $.type_parameters)),
      optional(field('parameters', $.parameters)),
      optional(seq(
        ':',
        field('return_type', $.type),
        optional(field('return_constraints', $.where_clause)),
      )),
      optional(field('contract', $.contract)),
      optional(seq('=', field('body', $._expression))),
    )),

    // `(enemy_id: EnemyId, damage: Int where Range(1, 20))`. In an adapter,
    // types can be left out: `fn draw(sprite, at) = ...`.
    parameters: $ => seq('(', optional(seq(
      $.parameter,
      repeat(seq(',', $.parameter)),
      optional(','),
    )), ')'),

    parameter: $ => seq(
      field('name', $._name),
      optional(seq(':', field('type', $.type), repeat(seq('or', field('type', $.type))))),
      optional(field('constraints', $.where_clause)),
    ),

    // Preconditions, parameter constraints and the ports a body `calls`.
    contract: $ => seq('{', repeat(choice($.calls_clause, $.parameter_constraint, $._expression)), '}'),

    // `damage where Range(1, 20)`, in a contract.
    parameter_constraint: $ => seq(field('name', $._name), field('constraints', $.where_clause)),

    calls_clause: $ => prec.right(seq('calls', $.path, repeat(seq(',', $.path)))),

    // `policy DropLoot when Room.EnemyDefeated { ... }`
    policy_declaration: $ => seq(
      'policy',
      field('name', $._type_name),
      'when',
      field('event', $.path),
      field('body', $.block),
    ),

    // Ports, adapters and repositories.

    port_declaration: $ => seq('port', field('name', $._type_name), '{', repeat($.function_declaration), '}'),

    // `adapter SpectrumScreen for Screen = { ... }`
    adapter_declaration: $ => seq(
      'adapter',
      field('name', $._type_name),
      'for',
      field('port', $.path),
      '=',
      '{',
      repeat(choice($.function_declaration, $.field_declaration)),
      '}',
    ),

    // `repository Rooms for Room`
    repository_declaration: $ => seq(
      'repository',
      field('name', $._type_name),
      'for',
      field('aggregate', $.path),
    ),

    // Types.

    // `Int`, `Room.Error`, `List<Enemy>`, `Result<Unit, GameError>`, `String<32>`
    type: $ => prec.right(seq(
      typeName($),
      repeat(seq('.', typeName($))),
      optional(field('arguments', $.type_arguments)),
    )),

    type_arguments: $ => seq('<', choice($.integer, seq($.type, repeat(seq(',', $.type)))), '>'),

    type_parameters: $ => seq(
      '<',
      alias($.identifier, $.type_identifier),
      repeat(seq(',', alias($.identifier, $.type_identifier))),
      '>',
    ),

    // Expressions.

    _expression: $ => choice(
      $._name,
      $.integer,
      $.float,
      $.string,
      $.boolean,
      $.parenthesized_expression,
      $.block,
      $.list_expression,
      $.map_expression,
      $.if_expression,
      $.match_expression,
      $.loop_expression,
      $.break_expression,
      $.issue_expression,
      $.emit_expression,
      $.native_call,
      $.lambda_expression,
      $.field_expression,
      $.call_expression,
      $.try_expression,
      $.with_expression,
      $.record_expression,
      $.unary_expression,
      $.binary_expression,
    ),

    binary_expression: $ => {
      const table = [
        [PREC.or_else, 'or'],
        [PREC.or, '||'],
        [PREC.and, '&&'],
        [PREC.equality, choice('==', '!=')],
        [PREC.comparison, choice('<', '<=', '>', '>=')],
        [PREC.bit_or, '|'],
        [PREC.bit_xor, '^'],
        [PREC.bit_and, '&'],
        [PREC.shift, choice('<<', '>>')],
        [PREC.additive, choice('+', '-')],
        [PREC.multiplicative, choice('*', '/', '%')],
      ];
      return choice(...table.map(([precedence, operator]) => prec.left(/** @type {number} */ (precedence), seq(
        field('left', $._expression),
        field('operator', /** @type {RuleOrLiteral} */ (operator)),
        field('right', $._expression),
      ))));
    },

    unary_expression: $ => prec(PREC.unary, seq(
      field('operator', choice('!', '-', '~')),
      field('operand', $._expression),
    )),

    try_expression: $ => prec(PREC.try, seq($._expression, '?')),

    field_expression: $ => prec(PREC.postfix, seq(
      field('value', $._expression),
      '.',
      field('field', fieldName($)),
    )),

    call_expression: $ => prec(PREC.postfix, seq(
      field('function', $._expression),
      field('arguments', $.arguments),
    )),

    // `enemy with { hit_points: remaining }`
    with_expression: $ => prec(PREC.postfix, seq(
      field('value', $._expression),
      'with',
      field('fields', $.field_values),
    )),

    // `Position { x: 0, y: 0 }` or `Room.Enemy { ... }`
    record_expression: $ => prec.dynamic(1, seq(
      field('name', choice($._name, $.field_expression)),
      field('fields', $.field_values),
    )),

    // `(start, amount: 100)`
    arguments: $ => seq('(', optional(seq(
      $._argument,
      repeat(seq(',', $._argument)),
      optional(','),
    )), ')'),

    _argument: $ => choice($.named_argument, $._expression),

    named_argument: $ => prec.right(seq(field('name', $._name), ':', field('value', $._expression))),

    // `{ x: 10, y: 20 }`, or `{ enemy_id }` for `{ enemy_id: enemy_id }`
    field_values: $ => seq('{', repeat(seq($.field_value, optional(','))), '}'),

    field_value: $ => prec.right(seq(
      field('name', fieldName($)),
      optional(seq(':', field('value', $._expression))),
    )),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    // `{ ... }`: one statement per line, the last line being the result.
    block: $ => seq('{', repeat($._statement), '}'),

    _statement: $ => choice($.let_statement, $.set_statement, $._expression),

    let_statement: $ => prec.right(seq(
      'let',
      field('name', $._name),
      optional(seq(':', field('type', $.type))),
      '=',
      field('value', $._expression),
    )),

    set_statement: $ => prec.right(seq('set', field('name', $._name), '=', field('value', $._expression))),

    // `if condition { ... }`: a guard with no `else`.
    if_expression: $ => seq('if', field('condition', $._expression), field('body', $.block)),

    // `match kind { Coins(count) -> count * 10 }`
    match_expression: $ => seq(
      'match',
      field('subject', $._expression),
      '{',
      repeat(seq($.match_arm, optional(','))),
      '}',
    ),

    match_arm: $ => prec.right(seq(field('pattern', $.pattern), '->', field('value', $._expression))),

    // `Coins(count)`, `Item.Gold(amount)`, `None` or `_`
    pattern: $ => seq(
      field('name', $.path),
      optional(seq('(', repeat(seq(nameAs($, 'binding'), optional(','))), ')')),
    ),

    // `loop { ... }` or `loop (let i where Range(0, 99)) { ... }`
    loop_expression: $ => seq('loop', optional(field('counter', $.loop_counter)), field('body', $.block)),

    loop_counter: $ => seq('(', 'let', field('name', $._name), field('constraints', $.where_clause), ')'),

    break_expression: _ => 'break',

    // `issue Attack(enemy_id: target, damage: 5) on room`. A `?` after the
    // target applies to the issue.
    issue_expression: $ => prec(PREC.issue, seq(
      'issue',
      field('command', $._name),
      optional(field('arguments', $.arguments)),
      'on',
      field('target', $._expression),
    )),

    // `emit EnemyDamaged { enemy_id, remaining }`
    emit_expression: $ => prec.right(seq(
      'emit',
      field('event', $.path),
      optional(field('fields', $.field_values)),
    )),

    // `native zx_print_at(at.y / 8, at.x / 8, text)`
    native_call: $ => seq('native', field('function', $.identifier), field('arguments', $.arguments)),

    // `enemy -> enemy.hit_points > 0`
    lambda_expression: $ => prec.right(PREC.lambda, seq(
      field('parameter', $._name),
      '->',
      field('body', $._expression),
    )),

    // `[a, b]`, `[]`
    list_expression: $ => seq('[', optional(seq(
      $._expression,
      repeat(seq(',', $._expression)),
      optional(','),
    )), ']'),

    // `[key: value, ...]` or `[:]`
    map_expression: $ => seq('[', choice(
      ':',
      seq($.map_entry, repeat(seq(',', $.map_entry)), optional(',')),
    ), ']'),

    map_entry: $ => prec.right(seq(field('key', $._expression), ':', field('value', $._expression))),

    // Names and literals.

    // A name. `event` and `error` are names too, outside their declarations.
    _name: $ => choice($.identifier, alias('event', $.identifier), alias('error', $.identifier)),

    _type_name: $ => typeName($),

    identifier: _ => /[\p{L}_][\p{L}\p{N}_]*/,

    integer: _ => token(choice(/0x[0-9A-Fa-f_]+/, /0b[01_]+/, /[0-9][0-9_]*/)),

    float: _ => /[0-9][0-9_]*\.[0-9][0-9_]*/,

    // The compiler knows five escapes: \n, \r, \t, \" and \\.
    string: $ => seq(
      '"',
      repeat(choice(token.immediate(prec(1, /[^"\\]+/)), $.escape_sequence)),
      token.immediate('"'),
    ),

    escape_sequence: _ => token.immediate(/\\[nrt"\\]/),

    boolean: _ => choice('true', 'false'),
  },
});

/**
 * A name, `event` and `error` included, shown as the node `kind`.
 *
 * @param {GrammarSymbols<string>} $
 * @param {string} kind
 */
function nameAs($, kind) {
  return choice(
    alias($.identifier, $[kind]),
    alias('event', $[kind]),
    alias('error', $[kind]),
  );
}

/**
 * A name in a type position: `Room.Error`, `List`.
 *
 * @param {GrammarSymbols<string>} $
 */
function typeName($) {
  return nameAs($, 'type_identifier');
}

/**
 * A field's name after `.` or in a record, where a keyword is a name too.
 *
 * @param {GrammarSymbols<string>} $
 */
function fieldName($) {
  return choice(
    alias($.identifier, $.field_identifier),
    ...KEYWORDS.map(keyword => alias(keyword, $.field_identifier)),
  );
}
