package org.ohxorud.breom.jetbrains.lexer;

import com.intellij.lexer.FlexLexer;
import com.intellij.psi.tree.IElementType;
import org.ohxorud.breom.jetbrains.psi.BreomTypes;
import com.intellij.psi.TokenType;

%%

%class BreomLexer
%implements FlexLexer
%public
%unicode
%function advance
%type IElementType



WHITE_SPACE=[ \t\n\r\f]+
IDENTIFIER=[a-z_][a-zA-Z0-9_]*
TYPE_IDENTIFIER=[A-Z][a-zA-Z0-9]*
DECIMAL=[0-9][0-9_]*
HEX=0[xX][0-9a-fA-F_]+
OCTAL=0[oO][0-7_]+
BINARY=0[bB][01_]+
FLOAT=[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+
ESCAPE=\\[nrt\\\"\'0\{\}]|\\x[0-9a-fA-F]{2}|\\u\{[0-9a-fA-F]{1,6}\}

%%

<YYINITIAL> {
  "package"         { return BreomTypes.PACKAGE_KEYWORD; }
  "breom"           { return BreomTypes.BREOM_KEYWORD; }
  "entrypoint"      { return BreomTypes.ENTRYPOINT_KEYWORD; }
  "dep"             { return BreomTypes.DEP_KEYWORD; }
  "import"          { return BreomTypes.IMPORT_KEYWORD; }
  "as"              { return BreomTypes.AS_KEYWORD; }
  "pub"             { return BreomTypes.PUB_KEYWORD; }
  "mut"             { return BreomTypes.MUT_KEYWORD; }
  "define"          { return BreomTypes.DEFINE_KEYWORD; }
  "fn"              { return BreomTypes.FN_KEYWORD; }
  "return"          { return BreomTypes.RETURN_KEYWORD; }
  "defer"           { return BreomTypes.DEFER_KEYWORD; }
  "for"             { return BreomTypes.FOR_KEYWORD; }
  "range"           { return BreomTypes.RANGE_KEYWORD; }
  "if"              { return BreomTypes.IF_KEYWORD; }
  "else"            { return BreomTypes.ELSE_KEYWORD; }
  "struct"          { return BreomTypes.STRUCT_KEYWORD; }
  "interface"       { return BreomTypes.INTERFACE_KEYWORD; }
  "enum"            { return BreomTypes.ENUM_KEYWORD; }
  "attribute"       { return BreomTypes.ATTRIBUTE_KEYWORD; }
  "match"           { return BreomTypes.MATCH_KEYWORD; }
  "spawn"           { return BreomTypes.SPAWN_KEYWORD; }
  "wait"            { return BreomTypes.WAIT_KEYWORD; }
  "default"         { return BreomTypes.DEFAULT_KEYWORD; }
  "timeout"         { return BreomTypes.TIMEOUT_KEYWORD; }
  "to"              { return BreomTypes.TO_KEYWORD; }
  "point"           { return BreomTypes.POINT_KEYWORD; }
  "test"            { return BreomTypes.TEST_KEYWORD; }
  "new"             { return BreomTypes.NEW_KEYWORD; }
  "op"              { return BreomTypes.OP_KEYWORD; }
  "throw"           { return BreomTypes.THROW_KEYWORD; }
  "throws"          { return BreomTypes.THROWS_KEYWORD; }
  "catch"           { return BreomTypes.CATCH_KEYWORD; }
  "instead"         { return BreomTypes.INSTEAD_KEYWORD; }
  "void"            { return BreomTypes.VOID_KEYWORD; }
  "true"            { return BreomTypes.TRUE_KEYWORD; }
  "false"           { return BreomTypes.FALSE_KEYWORD; }
  "shl"             { return BreomTypes.SHL_KEYWORD; }
  "shr"             { return BreomTypes.SHR_KEYWORD; }
  "break"           { return BreomTypes.BREAK_KEYWORD; }
  "continue"        { return BreomTypes.CONTINUE_KEYWORD; }
  "in"              { return BreomTypes.IN_KEYWORD; }
  "Tuple"           { return BreomTypes.TUPLE_KEYWORD; }
  "chan"            { return BreomTypes.CHAN_KEYWORD; }
  "self"            { return BreomTypes.SELF_KEYWORD; }

  "=="              { return BreomTypes.EQEQ; }
  "!="              { return BreomTypes.NEQ; }
  "<="              { return BreomTypes.LTEQ; }
  ">="              { return BreomTypes.GTEQ; }
  "&&"              { return BreomTypes.ANDAND; }
  "||"              { return BreomTypes.OROR; }
  "+="              { return BreomTypes.PLUSEQ; }
  "-="              { return BreomTypes.MINUSEQ; }
  "*="              { return BreomTypes.STAREQ; }
  "/="              { return BreomTypes.SLASHEQ; }
  "%="              { return BreomTypes.PERCENTEQ; }
  "&="              { return BreomTypes.ANDEQ; }
  "|="              { return BreomTypes.OREQ; }
  "^="              { return BreomTypes.XOREQ; }
  "->"              { return BreomTypes.ARROW; }
  "=>"              { return BreomTypes.FATARROW; }
  "::"              { return BreomTypes.COLONCOLON; }
  ":="              { return BreomTypes.COLONEQ; }
  "<<"              { return BreomTypes.LSHIFT; }

  "+"               { return BreomTypes.PLUS; }
  "-"               { return BreomTypes.MINUS; }
  "*"               { return BreomTypes.STAR; }
  "/"               { return BreomTypes.SLASH; }
  "%"               { return BreomTypes.PERCENT; }
  "="               { return BreomTypes.EQ; }
  "<"               { return BreomTypes.LT; }
  ">"               { return BreomTypes.GT; }
  "!"               { return BreomTypes.NOT; }
  "&"               { return BreomTypes.AND; }
  "|"               { return BreomTypes.OR; }
  "^"               { return BreomTypes.XOR; }
  "~"               { return BreomTypes.TILDE; }
  "?"               { return BreomTypes.QUESTION; }

  "("               { return BreomTypes.LPAREN; }
  ")"               { return BreomTypes.RPAREN; }
  "{"               { return BreomTypes.LBRACE; }
  "}"               { return BreomTypes.RBRACE; }
  "["               { return BreomTypes.LBRACKET; }
  "]"               { return BreomTypes.RBRACKET; }
  ","               { return BreomTypes.COMMA; }
  "."               { return BreomTypes.DOT; }
  ":"               { return BreomTypes.COLON; }
  ";"               { return BreomTypes.SEMICOLON; }
  "@"               { return BreomTypes.AT; }
  "_"               { return BreomTypes.UNDERSCORE; }

  "//"[^\r\n]*      { return BreomTypes.LINE_COMMENT; }
  "/*" ~"*/"        { return BreomTypes.BLOCK_COMMENT; }


  f\"([^\\\"\r\n]|\\[^\r\n])*\"  { return BreomTypes.FSTRING_LITERAL; }

  \"([^\\\"\r\n]|\\[^\r\n])*\"   { return BreomTypes.STRING_LITERAL; }

  \'([^\\\'\r\n]|\\[^\r\n])+\'   { return BreomTypes.CHAR_LITERAL; }

  \"\"\"([^\"]|\"[^\"]|\"\"[^\"])*\"\"\" { return BreomTypes.MULTILINE_STRING; }

  {FLOAT}           { return BreomTypes.FLOAT_LITERAL; }
  {HEX}             { return BreomTypes.INTEGER_LITERAL; }
  {OCTAL}           { return BreomTypes.INTEGER_LITERAL; }
  {BINARY}          { return BreomTypes.INTEGER_LITERAL; }
  {DECIMAL}         { return BreomTypes.INTEGER_LITERAL; }

  {TYPE_IDENTIFIER} { return BreomTypes.TYPE_IDENTIFIER; }
  {IDENTIFIER}      { return BreomTypes.IDENTIFIER; }

  {WHITE_SPACE}     { return TokenType.WHITE_SPACE; }
}

[^]                 { return TokenType.BAD_CHARACTER; }
