# Breom 한국어 문서

이 문서는 현재 코드베이스(`src/`, `std/`, `tests/`) 기준으로 정리한 Breom 사용 가이드입니다.

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

- `init`: `project.breom`과 `main.brm` 템플릿 생성
- `run`: 프로젝트 또는 단일 `.brm` 파일 실행
- `test`: `*_test.brm`의 `@test` 함수 실행
- `build`: 파싱/타입체크/코드생성 검증(실행 없음)
- `lsp`: Language Server 실행

테스트 타겟 예시:

- `breom test`
- `breom test .`
- `breom test ./...`
- `breom test tests/...`
- `breom test tests/test/main_test.brm`
- `breom test tests/... --filter map -v`

테스트 종료 코드(사용자 관점):

- `0`: 테스트 성공
- `1`: 테스트 실패 또는 단일 타겟에서 매칭 테스트 없음
- `2`: 테스트 로드/컴파일 오류

### 1.1 에디터 실행 명령

- VSCode 확장:
  - `Breom: Run Current File`
  - `Breom: Run Project`
- JetBrains 플러그인:
  - `Run Breom File`
  - `Run Breom Project`
  - Breom Run Configuration (Run/Debug 드롭다운)
- 모든 에디터 실행 액션은 동일한 CLI 계열 명령(`breom run`)을 사용합니다.

## 2. 프로젝트 구조

`project.breom`은 Breom 문법으로 파싱됩니다.

```breom
breom 0.1.0
package hello
entrypoint main.brm
```

- `breom`: 필수 Breom 언어 버전 (`<major>.<minor>.<patch>`)
- `package`: 루트 패키지명
- `entrypoint`: 엔트리 파일 경로 (`생략 시 main.brm`)
- 외부 의존성은 `dep "<repo>" "<tag>"`로 선언
- `project.breom`에서는 `import`를 사용할 수 없음
- 해석된 의존성 커밋은 `lock.breom`에 저장
- 소스 탐색:
  - 프로젝트 루트의 모든 `.brm`
  - 상위 디렉토리로 거슬러 올라가며 발견한 `std/` 하위 `.brm`
- 테스트/파서 fixture 제외 규칙:
  - `*_test.brm`은 `run/build`에서 제외
  - `*_fail.brm`은 소스 탐색에서 제외

패키지명 추론:

- 루트 바로 아래 파일: `project.breom`의 `package` 사용
- 하위 폴더 파일: 상대 경로를 `.`로 변환 (`foo/bar/x.brm` -> `foo.bar`)
- `std/` 하위 파일: `std` 아래 폴더 경로 사용 (`std/net/http/...` -> `net.http`)

## 3. 파일 문법 기본

```breom
import net.http
import file.io as io
```

- 파일에 `package`를 직접 쓰지 않습니다(경로 기반 추론)
- 주석:
  - `// ...`
  - `/* ... */`

## 4. 선언

### 4.1 상수/변수

```breom
define MaxConn = 100
define Pi Float = 3.14

count Int = 10
mut total Int = 0
name := "breom"
```

- 변수 선언:
  - `[pub] [mut] <name> <type> = <expr>`
  - `[pub] [mut] <name> := <expr>`
- 복합 대입 지원: `+= -= *= /= %= &= |= ^=`

### 4.2 함수

```breom
fn add(a Int, b Int) Int {
    return a + b
}

fn divide(a Int, b Int) Int throws {
    if b == 0 { throw new Error("division by zero") }
    return a / b
}
```

- `throws`로 에러 가능 함수 선언
- `main` 제약:
  - 인자 없음
  - 반환 `Int`, `Void`, 또는 생략

### 4.3 제네릭

```breom
fn pickInt<T: Int>(x T) Int {
    return 1
}

struct Box<T> {
    value T
}
```

- 함수/구조체 제네릭 지원
- 제네릭 제약(`T: Int`)은 호출/인스턴스화 시점에서 검증

### 4.4 속성(Attribute)

```breom
attribute bench(iter Int, warmup Int)

@bench(100, 10)
@test
fn sample() {
    assert(true)
}
```

- 선언: `attribute name` 또는 `attribute name(param Type, ...)`
- 사용: `@name` 또는 `@name(expr, ...)`
- 표준 내장 속성:
  - `@test`
  - `@compile_fail("...")`
  - `@parser_fail("path")`
- 선언되지 않은 속성, 인자 개수 불일치 시 컴파일 에러
- `@test()`처럼 0-인자 속성에 괄호는 허용되지 않음

### 4.5 구조체

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

- 멤버: 필드, `new`, `default`, `fn`, `op`, `to`
- 가시성: `pub` / 기본 private
- `Type.default()` 호출 가능
  - `default()` 멤버가 있으면 해당 구현 사용
  - 없으면 필드 타입 기본값(0/false/"" 등) 기반 자동 생성

### 4.6 인터페이스/상속/point 필드

`interface`는 메서드 시그니처(또는 기본 구현) 묶음을 선언할 때 사용합니다.

```breom
interface Named {
    fn name(self) String
}
```

- 기본 형식: `[pub] interface <Name>[<T, ...>] { ... }`
- 멤버는 아래를 지원합니다.
  - 시그니처만 선언: `fn method(self, arg Type) Ret`
  - 기본 구현 포함: `fn method(...) Ret { ... }`
  - 변환 선언: `to Type` 또는 `as Type`
  - 변환 기본 구현: `to Type { ... }` 또는 `as Type { ... }`
- 인터페이스 멤버에서 파라미터 문법은 메서드와 동일하며 `self`를 사용할 수 있습니다.

인터페이스와 구조체 상속/구현은 같은 상속 목록(`:` 뒤)으로 선언합니다.

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

- 상속 목록 규칙:
  - `struct Child: ParentA, ParentB` 형태의 다중 concrete 부모 상속 지원
  - 인터페이스는 같은 목록에 `,`로 함께 선언 가능 (`: Parent, Named, ...`)
  - 다중 부모에서 동일 메서드/변환이 충돌하면 컴파일 에러
  - 충돌 예외는 `@resolve_inherit("method:<name>", "<Parent>")`, `@resolve_inherit("conv:<Type>", "<Parent>")`로 지정

`point` 필드는 포함(embedding)된 구조체의 멤버를 승격해서 접근할 때 사용합니다.

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

- 선언 형식: `point <field_name> <StructType>`
- 승격 동작:
  - 필드 접근 승격: `obj.x` -> `obj.info.x`
  - 메서드 호출 승격: `obj.inc()` -> `obj.info.inc()`
- 같은 이름이 여러 `point` 경로에서 충돌하면 모호성 에러가 발생할 수 있습니다.

### 4.7 enum

```breom
enum MaybeInt {
    Some(Int)
    None
}
```

- enum variant는 payload를 가질 수 있습니다.
- `match`에서 `Some(x)` 형태의 enum 패턴 매칭을 지원합니다.

## 5. 타입

기본 타입:

- `Int`, `Int8`, `Int16`, `Int32`, `Int64`
- `UInt`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Byte`
- `Float`, `Float32`, `Float64`
- `Bool`, `String`, `Char`, `Void`, `Error`

복합 타입:

- 정적 배열: `[N]T`
- 동적 배열: `[]T`
- 튜플: `Tuple[T1, T2, ...]`
- 채널: `Channel<T>`
- 함수 타입: `fn(T1, T2) TRet`
- 제네릭 타입: `Type<T, U>`

## 6. 리터럴

- 정수: `10`, `0xFF`, `0o77`, `0b1010`, `1_000`
- 실수: `3.14`, `1e9`
- 문자열: `"hello"`, 멀티라인 `"""..."""`
- f-string: `f"hello {name}"`
- 문자: `'a'`
- 불리언: `true`, `false`
- void: `Void`
- 컬렉션:
  - 동적 배열: `[1, 2, 3]`
  - 반복 배열: `[2; 5]`
  - 맵: `("a": 1, "b": 2)`
  - 셋: `{1, 2, 3}`
  - 튜플: `(1, "a")`

정적 배열 컨텍스트 예:

```breom
nums [4]Int = [1, 2]
// => [1, 2, 0, 0]으로 패딩
```

## 7. 식/연산자

연산자:

- 산술: `+ - * / %`
- 비교: `== != < <= > >=`
- 논리: `&& || !`
- 비트: `& | ^ ~ shl shr`
- 삼항: `cond ? a : b`
- 캐스트: `expr as Type`

후위 연산:

- 호출: `f(x)`
- 멤버 접근: `obj.field`
- 인덱스: `arr[i]`
- 에러 전파: `expr?`
- 채널 송신: `ch << value`
- 에러 대체: `expr instead fallback`
- 에러 처리: `expr catch { ... }`

기타:

- 채널 수신: `<< ch`

## 8. 문장/제어 흐름

- `return`, `throw`, `defer`
- `if / else if / else`
- `for`
  - `for { ... }`
  - `for cond { ... }`
  - `for 10 { ... }`
  - `for i := range xs { ... }`
  - `for i, v := range xs { ... }`
- `match` (리터럴/바인딩/와일드카드/enum 패턴)
- `spawn`, `wait`
- `break`, `continue`
- `instead <expr>` (`catch` 블록 내 대체값 지정)

`wait` arm:

- `v := << ch => { ... }`
- `default => { ... }`
- `timeout(ms) => { ... }`

## 9. 람다

```breom
f := (x Int) -> x + 1
g := (x Int, y Int) Int -> {
    return (x + y) * 2
}
```

- 표현식 본문/블록 본문 모두 지원
- 파라미터 타입/반환 타입 생략 가능

## 10. 에러 처리

핵심:

- 함수 선언: `... throws`
- 에러 생성: `new Error("msg")`
- 발생: `throw err`
- 전파: `expr?`
- 처리:
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

## 11. 동시성/네트워크

### 11.1 채널/스레드

- 채널 생성: `Channel<T>.new(buffer)`
- 송신: `ch << v`
- 수신: `<< ch`
- 동시 실행: `spawn ...`
- 이벤트 대기: `wait { ... }`

### 11.2 네트워크

- UDP
  - `net.udp.bind(port)`
  - `net.udp.send(socket, address, port, data)`
- TCP
  - `net.tcp.bind(port)`
  - `net.tcp.connect(address, port)`
  - `net.tcp.send(socket, conn, data)`
  - `net.tcp.recv(socket)`
- HTTP(`std/net/http`)
  - `net.http.listen(port, handler)`
  - `net.http.get(host, port, path)`
  - `net.http.request(host, port, req)`

## 12. 파일 I/O

- `file.io.read(path)`
- `file.io.read_byte_sum(path)`
- `file.io.write(path, data)`
- `file.io.append(path, data)`
- `file.io.exists(path)`
- `file.io.remove(path)`
- `file.io.mkdir(path)`
- `file.io.reader(path)` -> `Reader`
- `file.io.scanner(path)` -> `Scanner`

핸들 메서드:

- `Reader.read_all()`, `Reader.close()`
- `Scanner.has_next()`, `Scanner.next_line()`, `Scanner.close()`

## 13. 내장 함수/메서드

함수:

- `print(v)`, `println(v)`
  - `String` 또는 `StringConvertable`을 받습니다.
  - `StringConvertable`은 `to String`(또는 `as String`) 변환을 제공하는 인터페이스입니다.
- `len(v)` (`String`, 배열, `Map`, `Set`)
- `sleep(ms)`

테스트 전용(`breom test`):

- `assert(cond)`
- `fail(msg)`

컬렉션/문자열 메서드:

- `String.len()`
- 배열: `.len()`, `.push(x)`, `.pop()`, `.get(i)`
- 맵: `.len()`, `.get(k)`, `.set(k,v)`, `.contains(k)`, `.delete(k)`
- 셋: `.len()`, `.add(v)`, `.contains(v)`, `.remove(v)`

## 14. 테스트 시스템

파일 규칙:

- 테스트 파일은 반드시 `*_test.brm`

테스트 함수 규칙:

- `@test` 필수
- 파라미터 없음
- `throws` 금지
- 반환 `Void` 또는 생략

컴파일 실패 테스트:

```breom
@test
@compile_fail("Static array length mismatch")
fn static_array_len_mismatch_compile_error() Int {
    return static_array_sum([1, 2, 3])
}
```

- 함수 본문 코드가 컴파일 실패해야 통과
- 실패 메시지에 지정 문자열 포함 필요

파서/컴파일 fixture 실패 테스트:

```breom
@test
@parser_fail("fixtures/broken_syntax_fail.brm")
fn broken_syntax_parser_fail() {}
```

- fixture 경로는 현재 `_test.brm` 기준 상대 경로
- `*_fail.brm` 네이밍 권장

## 15. LSP 기능

`breom lsp`에서 제공:

- 진단(파싱 오류)
- Hover
- 자동완성(점 접근 포함)
- 정의로 이동
- 참조 찾기
- 문서 심볼(Outline)
- 워크스페이스 심볼 검색
- 시맨틱 토큰

LSP의 std 패키지 해석 순서:

- `BREOM_STD_PATH` 환경변수
- 워크스페이스 상위 경로에서 발견한 `std/`
- 둘 다 없으면 내장 std 스텁(고정 패키지 목록)

참고:

- 실제 std 소스가 없을 때도 자동완성/hover/진단은 계속 동작
- 이 경우 일부 정의로 이동은 제한될 수 있으며 경고가 표시됨
