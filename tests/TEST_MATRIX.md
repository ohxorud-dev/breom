# Breom Test Matrix

## Test Attribute Conventions

- Each suite directory should be a Breom project initialized with `breom init <name>` and include `project.breom`.
- Test functions must use built-in `@test` (zero-arg): use `@test`, not `@test()`.
- Custom attributes must be declared at top-level: `attribute name` or `attribute name(param Type, ...)`.
- Undeclared attributes and argument-count mismatches are compile errors.

## Compile-Fail Tests

- Use built-in `@compile_fail(contains)` together with `@test` for negative compiler tests.
- Write the failing code directly in the test function body.
- `contains` must be a string literal expected to appear in the compile error message.
- `@compile_fail` must be paired with `@test`.

## Parser-Fail Tests

- Use built-in `@parser_fail(path)` together with `@test` for fixture-based parse/compile failure checks.
- `path` is resolved relative to the current `_test.brm` file.
- Parser-fail fixtures should use `*_fail.brm` naming so source discovery skips them.
- `@parser_fail` must be paired with `@test`.

Example:

```breom
@test
@compile_fail(
    "Static array length mismatch"
)
fn static_array_len_mismatch_compile_error() Int {
    return static_array_sum([1, 2, 3])
}
```

## Explicitly Unsupported (Compile-Fail Guarded)

- `interface` / `inheritance` / `point` paths are fixture-guarded via `@parser_fail` (`tests/syntax/main_test.brm`)
- `match` enum-pattern paths emit explicit compile errors (`tests/syntax/main_test.brm`)

## Implemented And Regression-Covered

- `wait` default + timeout arms are regression-covered (`tests/concurrency/main_test.brm`)
- `for ... := range` continue-path index progression is regression-covered (`tests/loop/main_test.brm`)
- lambda indirect-call signature path is regression-covered (incl. float args/returns) (`tests/syntax/main_test.brm`)
- ternary parsing + postfix `?` coexistence is regression-covered (`tests/syntax/main_test.brm`)
- `?` propagation + defer interaction is regression-covered (`tests/error/main_test.brm`)
- generic struct literal base-resolution path is regression-covered (`tests/syntax/main_test.brm`)
