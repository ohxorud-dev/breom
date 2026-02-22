# Breom Documentation (English)

This document is a practical guide to Breom based on the current codebase (`src/`, `std/`, `tests/`).

## 1. CLI

```bash
breom init [name]
breom run
breom run <file.brm>
breom test [target] [--filter pattern] [--verbose]
breom build [path]
breom lsp
breom <file.brm>
```

- `init`: creates `project.breom` and `main.brm` templates
- `run`: runs a project or a single `.brm` file
- `test`: runs `@test` functions in `*_test.brm`
- `build`: validates parsing/type-checking/code generation (no execution)
- `lsp`: starts the Language Server

Test target examples:

- `breom test`
- `breom test .`
- `breom test ./...`
- `breom test tests/...`
- `breom test tests/test/main_test.brm`
- `breom test tests/... --filter map -v`

Test exit codes (user-facing):

- `0`: tests passed
- `1`: tests failed, or no matching tests in a single target
- `2`: test load/compile error

### 1.1 Editor run commands

- VSCode extension:
  - `Breom: Run Current File`
  - `Breom: Run Project`
- JetBrains plugin:
  - `Run Breom File`
  - `Run Breom Project`
  - Breom Run Configuration (Run/Debug dropdown)
- All editor run actions execute the same CLI command family (`breom run`).

## 2. Project Structure

`project.breom` is parsed using Breom syntax.

```breom
breom 0.1.0
package hello
entrypoint main.brm
```

- `breom`: required Breom language version (`<major>.<minor>.<patch>`)
- `package`: root package name
- `entrypoint`: entry file path (`main.brm` if omitted)
- External dependencies are declared with `dep "<repo>" "<tag>"`
- `import` is not allowed in `project.breom`
- Resolved dependency commits are stored in `lock.breom`
- Source discovery:
  - all `.brm` files in the project root
  - `.brm` files under `std/` discovered while traversing parent directories
- Test/parser fixture exclusion rules:
  - `*_test.brm` is excluded from `run/build`
  - `*_fail.brm` is excluded from source discovery

Package name inference:

- Files directly under the root use `package` from `project.breom`
- Files in subfolders use the relative path with `.` (`foo/bar/x.brm` -> `foo.bar`)
- Files under `std/` use the path under `std` (`std/net/http/...` -> `net.http`)

## 3. File Syntax Basics

```breom
import net.http
import file.io as io
```

- Do not write `package` directly in files (path-based inference)
- Comments:
  - `// ...`
  - `/* ... */`

## 4. Declarations

### 4.1 Constants/Variables

```breom
define MaxConn = 100
define Pi Float = 3.14

count Int = 10
mut total Int = 0
name := "breom"
```

- Variable declaration:
  - `[pub] [mut] <name> <type> = <expr>`
  - `[pub] [mut] <name> := <expr>`
- Compound assignments supported: `+= -= *= /= %= &= |= ^=`

### 4.2 Functions

```breom
fn add(a Int, b Int) Int {
    return a + b
}

fn divide(a Int, b Int) Int throws {
    if b == 0 { throw new Error("division by zero") }
    return a / b
}
```

- Use `throws` for functions that can raise errors
- `main` constraints:
  - no parameters
  - return type is `Int`, `Void`, or omitted

### 4.3 Generics

```breom
fn pickInt<T: Int>(x T) Int {
    return 1
}

struct Box<T> {
    value T
}
```

- Generic functions/structs are supported
- Generic constraints (`T: Int`) are validated at call/instantiation time

### 4.4 Attributes

```breom
attribute bench(iter Int, warmup Int)

@bench(100, 10)
@test
fn sample() {
    assert(true)
}
```

- Declaration: `attribute name` or `attribute name(param Type, ...)`
- Usage: `@name` or `@name(expr, ...)`
- Standard built-in attributes:
  - `@test`
  - `@compile_fail("...")`
  - `@parser_fail("path")`
- Compile error if attribute is undeclared or argument count mismatches
- Parentheses are not allowed for zero-arg attributes like `@test()`

### 4.5 Structs

```breom
struct User {
    pub id Int
    name String

    new(id Int, name String) {
        return User { id: id, name: name }
    }

    default() {
        return User { id: 1, name: "guest" }
    }

    pub fn hello(self) {
        println(self.name)
    }

    op + (other User) User {
        return self
    }

    to Int {
        return self.id
    }
}
```

- Members: fields, `new`, `default`, `fn`, `op`, `to`
- Visibility: `pub` / default private
- `Type.default()` is callable
  - uses explicit `default()` member if present
  - otherwise auto-generates from field type defaults (0/false/"", etc.)

### 4.6 Interfaces/Inheritance/point Fields

Use `interface` to declare method signatures (or default implementations).

```breom
interface Named {
    fn name(self) String
}
```

- Base form: `[pub] interface <Name>[<T, ...>] { ... }`
- Supported members:
  - signature-only: `fn method(self, arg Type) Ret`
  - default implementation: `fn method(...) Ret { ... }`
  - conversion declaration: `to Type` or `as Type`
  - conversion default implementation: `to Type { ... }` or `as Type { ... }`
- Interface parameter syntax matches methods and can include `self`

Interface implementation and struct inheritance use one inheritance list (after `:`).

```breom
struct BaseEntity {
    id Int
}

struct UserEntity: BaseEntity, Named {
    title String

    pub fn name(self) String {
        return self.title
    }
}
```

- Inheritance list rules:
  - supports multiple concrete parents with `struct Child: ParentA, ParentB`
  - interfaces can be listed together with `,` (`: Parent, Named, ...`)
  - conflicting inherited methods/conversions fail at compile time
  - conflicts can be resolved with `@resolve_inherit("method:<name>", "<Parent>")` and `@resolve_inherit("conv:<Type>", "<Parent>")`

`point` fields are for embedding and promoting member access.

```breom
struct PointInfo {
    x Int

    pub fn inc(self) Int {
        return self.x + 1
    }
}

struct UserEntity: BaseEntity, Named {
    point info PointInfo
    title String

    pub fn name(self) String {
        return self.title
    }
}

fn main() Int {
    user := UserEntity { id: 1, info: PointInfo { x: 12 }, title: "neo" }
    println(user.x as String)     // user.info.x
    println(user.inc() as String) // user.info.inc()
    return 0
}
```

- Declaration form: `point <field_name> <StructType>`
- Promotion behavior:
  - field promotion: `obj.x` -> `obj.info.x`
  - method promotion: `obj.inc()` -> `obj.info.inc()`
- Name collisions across multiple `point` paths can cause ambiguity errors

### 4.7 enum

```breom
enum MaybeInt {
    Some(Int)
    None
}
```

- Enum variants can carry payloads
- `match` supports enum pattern matching like `Some(x)`

## 5. Types

Primitive types:

- `Int`, `Int8`, `Int16`, `Int32`, `Int64`
- `UInt`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Byte`
- `Float`, `Float32`, `Float64`
- `Bool`, `String`, `Char`, `Void`, `Error`

Composite types:

- Static array: `[N]T`
- Dynamic array: `[]T`
- Tuple: `Tuple[T1, T2, ...]`
- Channel: `Channel<T>`
- Function type: `fn(T1, T2) TRet`
- Generic type: `Type<T, U>`

## 6. Literals

- Integer: `10`, `0xFF`, `0o77`, `0b1010`, `1_000`
- Float: `3.14`, `1e9`
- String: `"hello"`, multiline `"""..."""`
- f-string: `f"hello {name}"`
- Char: `'a'`
- Boolean: `true`, `false`
- void: `Void`
- Collections:
  - dynamic array: `[1, 2, 3]`
  - repeated array: `[2; 5]`
  - map: `("a": 1, "b": 2)`
  - set: `{1, 2, 3}`
  - tuple: `(1, "a")`

Static array context example:

```breom
nums [4]Int = [1, 2]
// => padded to [1, 2, 0, 0]
```

## 7. Expressions/Operators

Operators:

- Arithmetic: `+ - * / %`
- Comparison: `== != < <= > >=`
- Logical: `&& || !`
- Bitwise: `& | ^ ~ shl shr`
- Ternary: `cond ? a : b`
- Cast: `expr as Type`

Postfix operations:

- Call: `f(x)`
- Member access: `obj.field`
- Indexing: `arr[i]`
- Error propagation: `expr?`
- Channel send: `ch << value`
- Error fallback: `expr instead fallback`
- Error handling: `expr catch { ... }`

Other:

- Channel receive: `<< ch`

## 8. Statements/Control Flow

- `return`, `throw`, `defer`
- `if / else if / else`
- `for`
  - `for { ... }`
  - `for cond { ... }`
  - `for 10 { ... }`
  - `for i := range xs { ... }`
  - `for i, v := range xs { ... }`
- `match` (literal/binding/wildcard/enum patterns)
- `spawn`, `wait`
- `break`, `continue`
- `instead <expr>` (fallback value inside a `catch` block)

`wait` arms:

- `v := << ch => { ... }`
- `default => { ... }`
- `timeout(ms) => { ... }`

## 9. Lambdas

```breom
f := (x Int) -> x + 1
g := (x Int, y Int) Int -> {
    return (x + y) * 2
}
```

- Supports expression-body and block-body forms
- Parameter and return types can be omitted

## 10. Error Handling

Core concepts:

- Function declaration: `... throws`
- Error creation: `new Error("msg")`
- Throw: `throw err`
- Propagation: `expr?`
- Handling:
  - `expr instead fallback`
  - `expr catch { ... }` + `instead <expr>`

```breom
fn main() Int {
    v := divide(10, 0) catch {
        println(err)
        instead -1
    }
    return v
}
```

## 11. Concurrency/Networking

### 11.1 Channels/Threads

- Create channel: `Channel<T>.new(buffer)`
- Send: `ch << v`
- Receive: `<< ch`
- Concurrent execution: `spawn ...`
- Wait for events: `wait { ... }`

### 11.2 Networking

- UDP
  - `net.udp.bind(port)`
  - `net.udp.send(socket, address, port, data)`
- TCP
  - `net.tcp.bind(port)`
  - `net.tcp.connect(address, port)`
  - `net.tcp.send(socket, conn, data)`
  - `net.tcp.recv(socket)`
- HTTP (`std/net/http`)
  - `net.http.listen(port, handler)`
  - `net.http.get(host, port, path)`
  - `net.http.request(host, port, req)`

## 12. File I/O

- `file.io.read(path)`
- `file.io.read_byte_sum(path)`
- `file.io.write(path, data)`
- `file.io.append(path, data)`
- `file.io.exists(path)`
- `file.io.remove(path)`
- `file.io.mkdir(path)`
- `file.io.reader(path)` -> `Reader`
- `file.io.scanner(path)` -> `Scanner`

Handle methods:

- `Reader.read_all()`, `Reader.close()`
- `Scanner.has_next()`, `Scanner.next_line()`, `Scanner.close()`

## 13. Built-in Functions/Methods

Functions:

- `print(v)`, `println(v)`
  - Accepts `String` or `StringConvertable`
  - `StringConvertable` is an interface that provides `to String` (or `as String`)
- `len(v)` (`String`, arrays, `Map`, `Set`)
- `sleep(ms)`

Test-only (`breom test`):

- `assert(cond)`
- `fail(msg)`

Collection/string methods:

- `String.len()`
- Array: `.len()`, `.push(x)`, `.pop()`, `.get(i)`
- Map: `.len()`, `.get(k)`, `.set(k,v)`, `.contains(k)`, `.delete(k)`
- Set: `.len()`, `.add(v)`, `.contains(v)`, `.remove(v)`

## 14. Test System

File rules:

- Test files must be named `*_test.brm`

Test function rules:

- `@test` is required
- no parameters
- `throws` is not allowed
- return type is `Void` or omitted

Compile-failure test:

```breom
@test
@compile_fail("Static array length mismatch")
fn static_array_len_mismatch_compile_error() Int {
    return static_array_sum([1, 2, 3])
}
```

- Passes only if function body fails to compile
- Error message must include the specified text

Parser/compile fixture failure test:

```breom
@test
@parser_fail("fixtures/broken_syntax_fail.brm")
fn broken_syntax_parser_fail() {}
```

- Fixture path is relative to the current `_test.brm` file
- `*_fail.brm` naming is recommended

## 15. LSP Features

Provided by `breom lsp`:

- Diagnostics (parse errors)
- Hover
- Autocomplete (including dot access)
- Go to definition
- Find references
- Document symbols (Outline)
- Workspace symbol search
- Semantic tokens

std package resolution order in LSP:

- `BREOM_STD_PATH` environment variable
- `std/` discovered while walking up from the workspace root
- Embedded std stubs when neither source is available

Notes:

- Autocomplete/hover/diagnostics still work without physical std sources
- Some go-to-definition targets may be limited in stub mode, and a warning is shown
