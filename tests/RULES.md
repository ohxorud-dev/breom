# Test Rules

## File layout

- Each test suite should live in its own folder under `tests/`.
- Each suite folder should be initialized as a Breom project (`breom init <name>`) and include `project.breom`.
- Test functions must be placed in `*_test.brm` files.
- Non-test/helper functions must be placed in `main.brm` (or another non-`*_test.brm` file) in the same folder.

## Test function requirements

- Test functions must declare the `@test` attribute.
- `@test` is a built-in zero-arg attribute, so use `@test` (not `@test()`).
- Test functions must take no parameters.
- Test functions must not use `throws`.
- Test functions should return `Void` (no explicit return value).

## Attribute rules

- Custom attributes must be declared at top-level before use: `attribute name` or `attribute name(param Type, ...)`.
- Undeclared attributes cause compile errors.
- Attributes with parameters must be used with matching argument count: `@name(expr, ...)`.

## Naming and style

- Use clear, behavior-focused names (for example: `for_range_index_only_accumulates_indices`).
- Keep assertions deterministic and avoid timing-fragile expectations where possible.

## Execution

- Run a single suite:
  - `target/debug/breom test tests/<suite>/main_test.brm`
- Run with filter:
  - `target/debug/breom test tests/<suite>/main_test.brm --filter <keyword>`
