#[derive(Debug, PartialEq, Eq)]
enum Stmt {
    FeatureDecl,
    DiacriticDecl,
    SymbolDecl,
    ClassDecl,
    ElementDecl,
    SyllableDecl,
    Demonanizer,
    InterRomanizer,
    Romanizer,
    ChangeRule,
    StandardExpression,
}

peg::parser!{
  grammar lsc() for str {
    // lscFile: (WHITESPACE | NEWLINE*) statement? (NEWLINE+ statement)* (WHITESPACE | NEWLINE*) EOF;
    pub rule lsc_file() -> Vec<Stmt>
      = _ r:statement()* { r }

    // fragment COMMENT_START: '#';
    // COMMENT: (WHITESPACE? COMMENT_START ~[\n\r]*) -> skip;
    rule comment() = quiet!{ whitespace()? "#" [^ '\n' | '\r']* }
    // WHITESPACE: ~[\P{White_Space}\r\n]+;
    rule whitespace() = quiet!{ [' ' | '\t' | '\r' | '\n']+ }
    // NEWLINE: WHITESPACE? ('\r\n' | '\n') WHITESPACE?;
    rule newline() = quiet!{ ['\r' | '\n'] }
    rule _ = (comment()? whitespace())*
    // NUMBER: DIGIT+;
    rule number() = ['0'..='9']+
    // NAME: CHAR+;
    rule sname() = ['A'..='Z' | 'a'..='z' | '0'..='9']+
    // fragment ANY: ('\\' .) | ~[ \\,.=>()*[\]{}+?/\-_:!~$@#&\n\r];
    rule any() = ("\\" [_]) / [^ '\\' | ',' | '.' | '=' | '>' | '(' | ')'
                               |  '*' | '[' | ']' | '{' | '}' | '+' | '?'
                               |  '/' | '-' | '_' | ':' | '!' | '~' | '$'
                               |  '@' | '#' | '&' | '\n' | '\r' | ' ' ]
    // STR1: ANY;
    rule sstr1() = any()
    // STR: ANY+;
    rule sstr() = any() +
    // LIST_SEP: ',' WHITESPACE?;
    // CLASS_SEP: ',' (WHITESPACE | NEWLINE)?;
    // CHANGE: WHITESPACE? '=>' (WHITESPACE | NEWLINE)?;
    // CONDITION: WHITESPACE? '/' (WHITESPACE | NEWLINE)?;
    // EXCLUSION: WHITESPACE? '//' (WHITESPACE | NEWLINE)?;
    // ANCHOR: '_';
    // O_PAREN: '(' WHITESPACE?;
    // C_PAREN:  WHITESPACE? ')';
    // NULL: '*';
    // MATRIX_START: '[' WHITESPACE?;
    // MATRIX_END:  WHITESPACE? ']';
    // LIST_START: '{' WHITESPACE?;
    // CLASS_START: '{' NEWLINE?;
    // LIST_END:  WHITESPACE? '}';
    // AT_LEAST_ONE: '+';
    // OPTIONAL: '?';
    // HYPHEN: '-';
    // RULE_START: ':';
    // DOUBLE_COLON: WHITESPACE? '::' WHITESPACE?;
    // QMARK_COLON: WHITESPACE? '?:' WHITESPACE?;
    // INEXACT: '~';
    // NEGATION: '!';
    // SYLLABLE_BOUNDARY: '.';
    // WORD_BOUNDARY: '$';
    // BETWEEN_WORDS: '$$';
    // CLASSREF: '@';
    // INTERSECTION: '&';
    // INTERSECTION_NOT: '&!';
    // TRANSFORMING: '>';

    // statement:
    //   featureDecl | diacriticDecl | symbolDecl | classDecl | elementDecl | syllableDecl |
    //   deromanizer | interRomanizer | romanizer | changeRule | standardExpression;
    rule statement() -> Stmt = featureDecl() / diacriticDecl() / symbolDecl() / classDecl()
                     / elementDecl() / syllableDecl() / deromanizer() / interRomanizer()
                     / romanizer() / changeRule() / standardExpression()

    // elementDecl: ELEMENT_DECL WHITESPACE name WHITESPACE ruleElement;
    // ELEMENT_DECL: 'Element' | 'element';
    rule elementDecl() -> Stmt = ("Element" / "element") _ name() _ ruleElement() { Stmt::ElementDecl }

    // classDecl: CLASS_DECL WHITESPACE name WHITESPACE (CLASS_START | LIST_START) classElement ((CLASS_SEP | LIST_SEP) classElement)* CLASS_SEP? LIST_END;
    rule classDecl() -> Stmt = ("Class" / "class") _ name() _ "{" _ classElement() ** ("," _) ","? "}" _ { Stmt::ClassDecl }
    // classElement: elementRef | text;
    rule classElement() = elementRef() / text()

    // featureDecl:
    //     FEATURE_DECL WHITESPACE (
    //         (plusFeature (LIST_SEP plusFeature)*) |
    //         ((featureModifier WHITESPACE)? name WHITESPACE? O_PAREN (nullAlias LIST_SEP)? featureValue (LIST_SEP featureValue)* C_PAREN)
    //     );
    // featureModifier: SYLLABLE_FEATURE;
    rule featureDecl() -> Stmt =
        ("Feature" / "feature") _ (
            plusFeature() ** ("," _ )
            / (featureModifier()? name() _ "(" _ (nullAlias() "," _)? featureValue() ++ ("," _) ")" _)
        ) { Stmt::FeatureDecl }

    // plusFeature: (featureModifier WHITESPACE)? AT_LEAST_ONE? name;
    rule plusFeature() = featureModifier()? "+"? _ name() _
    rule featureModifier() = "(Syllable)" / "syllable"

    // nullAlias: NULL featureValue;
    rule nullAlias() = "*" _ featureValue()

    // diacriticDecl:
    //     DIACRITIC_DECL WHITESPACE text WHITESPACE
    //     (diacriticModifier WHITESPACE)* matrix (WHITESPACE diacriticModifier)*;
    rule diacriticDecl() -> Stmt = ("Diacritic" / "diatritic") _ text() _ diacriticModifier()* matrix() diacriticModifier()* { Stmt::DiacriticDecl }

    // diacriticModifier: DIA_BEFORE | DIA_FIRST | DIA_FLOATING;
    rule diacriticModifier() = ("(Before)" / "(before)" / "(First)" / "(first)" / "(Floating)" / "(floating)") _
    // symbolDecl: SYMBOL_DECL WHITESPACE symbolName ((LIST_SEP symbolName)* | WHITESPACE matrix);
    rule symbolDecl() -> Stmt = ("Symbol" / "symbol") _ symbolName() _ (("," _ symbolName())* / matrix()) { Stmt::SymbolDecl }
    // symbolName: text;
    rule symbolName() = text() _

    // syllableDecl:
    //     SYLLABLE_DECL RULE_START (NEWLINE+ (EXPLICIT_SYLLABLES | CLEAR_SYLLABLES) | (NEWLINE+ syllableExpression)+);
    rule syllableDecl() -> Stmt = ("Syllable" / "syllable") _ ":" _ (("Explicit" / "explicit") _ / ("Clear" / "clear") _ / syllableExpression()+) { Stmt::SyllableDecl }

    // syllableExpression: syllablePattern (CHANGE matrix)? compoundEnvironment?;
    rule syllableExpression() = syllablePattern() ("=>" _ matrix())? compoundEnvironment()?

    // syllablePattern: structuredPattern | ruleElement;
    rule syllablePattern() = structuredPattern() / ruleElement()

    // structuredPattern:
    //     (reluctantOnset QMARK_COLON)?
    //     unconditionalRuleElement DOUBLE_COLON
    //     unconditionalRuleElement (DOUBLE_COLON unconditionalRuleElement)?;
    rule structuredPattern() = (reluctantOnset() "?:" _)? unconditionalRuleElement() "::" _ unconditionalRuleElement() ("::" _ unconditionalRuleElement())?
    // reluctantOnset: unconditionalRuleElement;
    rule reluctantOnset() = unconditionalRuleElement()

    // deromanizer: DEROMANIZER (WHITESPACE LITERAL)? RULE_START NEWLINE+ block;
    rule deromanizer() -> Stmt = ("Deromanizer" / "deromanizer") _ ("Literal" / "literal") _ ":" _ block() { Stmt::Demonanizer }

    // romanizer: ROMANIZER (WHITESPACE LITERAL)? RULE_START NEWLINE+ block;
    rule romanizer() -> Stmt = ("Romanizer" / "romanizer") _ ("Literal" / "literal") _ ":" _ block() { Stmt::Romanizer }
    // interRomanizer: ROMANIZER HYPHEN ruleName (WHITESPACE LITERAL)? RULE_START NEWLINE+ block;
    rule interRomanizer() -> Stmt = ("Romanizer-" / "romanizer-") ruleName() _ ("Literal" / "literal") _ ":" _ block() { Stmt::InterRomanizer }

    // changeRule: ruleName (WHITESPACE changeRuleModifier)* RULE_START? NEWLINE+ block;
    rule changeRule() -> Stmt = ruleName() _ (changeRuleModifier())* ":"? _ block() { Stmt::ChangeRule }

    // filter: elementRef | fancyMatrix;
    rule filter() = elementRef() / fancyMatrix()

    // block: blockElement (NEWLINE+ blockType RULE_START (WHITESPACE | NEWLINE+) blockElement)*;
    rule block() = blockElement() _ (blockType() ":" _ blockElement())*

    // blockElement: expressionList | O_PAREN NEWLINE* block NEWLINE* C_PAREN;
    rule blockElement() = expressionList() / "(" _ block() ")" _

    // blockType: (ALL_MATCHING | FIRST_MATCHING) (WHITESPACE changeRuleModifier)*;
    rule blockType() = (("Then" / "then") / ("Else" / "else")) _ changeRuleModifier()*

    // changeRuleModifier: filter | keywordModifier;
    rule changeRuleModifier() = filter() / keywordModifier()

    // keywordModifier: LTR | RTL | PROPAGATE | BLOCK | CLEANUP | NAME;
    rule keywordModifier() = (("ltr" / "Ltr") / ("Rtl" / "Rtl") / ("Propagate" / "propagate") / ("Defer" / "defer") / ("Cleanup" / "cleanup")) _ / name()
    // expressionList: expression (NEWLINE+ expression)*;
    rule expressionList() = expression()*
    // ruleName: name (HYPHEN (name | NUMBER))*;
    rule ruleName() = name() ("-" (name() / number()))*
    // expression: keywordExpression | blockRef | standardExpression;
    rule expression() = keywordExpression() / blockRef() / standardExpression()
    // keywordExpression: UNCHANGED | OFF;
    rule keywordExpression() = ("Unchanged" / "Unchanged") / ("Off" / "off") _
    // blockRef: RULE_START ruleName;
    rule blockRef() = ":" ruleName()
    // standardExpression: from CHANGE to compoundEnvironment?;
    // from: ruleElement;
    // to: unconditionalRuleElement;
    rule standardExpression() -> Stmt = ruleElement() "=>" _ unconditionalRuleElement() compoundEnvironment()? { Stmt::StandardExpression }

    // ruleElement: unconditionalRuleElement compoundEnvironment?;
    rule ruleElement() = unconditionalRuleElement() compoundEnvironment()?
    // unconditionalRuleElement: bounded | interfix | negated | postfix | simple | sequence;
    rule unconditionalRuleElement() = bounded() / interfix() / negated() / postfix() / simple() / sequence()

    // // "Bounded" elements have a clear start and end symbol
    // bounded: group | list;
    // group: O_PAREN ruleElement C_PAREN;
    // list: LIST_START ruleElement (LIST_SEP ruleElement)* LIST_END;
    rule bounded() = "(" _ ruleElement() ")" _
                   / "{" _ ruleElement() ++ ("," _) "}" _

    // // "Free" elements have sub-elements floating free amid whitespace
    // sequence: freeElement (WHITESPACE freeElement)+;
    // freeElement: bounded | interfix | negated | postfix | simple;
    rule sequence() = (bounded() / interfix() / negated() / postfix() / simple())+

    // compoundEnvironment: condition | exclusion | (condition exclusion);
    rule compoundEnvironment() = condition() / exclusion() / (condition() exclusion())

    // condition: CONDITION (environment | environmentList);
    rule condition() = "/" _ (environment() / environmentList())
    // exclusion: EXCLUSION (environment | environmentList);
    rule exclusion() = "//" _ (environment() / environmentList())
    // environmentList: LIST_START environment (LIST_SEP environment)* LIST_END;
    rule environmentList() = "{" _ environment() ++ ("," _) "}" _
    // environment:
    //     (environmentBefore WHITESPACE)? ANCHOR (WHITESPACE environmentAfter)?
    //     | environmentBefore?;
    // environmentBefore: unconditionalRuleElement;
    // environmentAfter: unconditionalRuleElement;
    rule environment() = unconditionalRuleElement()? "_" _ unconditionalRuleElement()?
                       / unconditionalRuleElement()

    // // "Interfix" elements use a delimiter but no whitespace or boundary marker
    // interfix: interfixElement (interfixType interfixElement)+;
    // interfixType: INTERSECTION | INTERSECTION_NOT | TRANSFORMING;
    // interfixElement: bounded | negated | postfix | simple;
    rule interfix() = interfixElement() (("&" / "!&" / ">") _ interfixElement())+
    rule interfixElement() = bounded() / negated() / postfix() / simple()

    // // "Prefix" elements use a prefix operator
    // negated: NEGATION (bounded | simple);
    rule negated() = "!" (bounded() / simple())

    // // "Postfix" elements use a postfix operator
    // postfix: capture | repeater;
    rule postfix() = capture() / repeater()
    // capture: (bounded | negated | simple) captureRef;
    rule capture() = (bounded() / negated() / simple()) captureRef()
    // repeater: (bounded | simple) repeaterType;
    rule repeater() = (bounded() / simple()) repeaterType()

    // // "Simple" elements can't have other elements inside them
    // simple: anySyllable | elementRef | captureRef | fancyMatrix | empty | sylBoundary | boundary | betweenWords | text;
    // anySyllable: ANY_SYLLABLE;
    rule simple() = ("<Syl>" / "<syl>") _ / elementRef() / captureRef() / fancyMatrix() / empty()
                  / ("." _) / ("$" _) / ("$$" _) / text()
    // elementRef: CLASSREF name;
    rule elementRef() = "@" name()
    // captureRef: INEXACT? WORD_BOUNDARY SYLLABLE_BOUNDARY? NUMBER;
    rule captureRef() = "~"? "$" "."? number()

    // fancyMatrix: MATRIX_START fancyValue? (WHITESPACE fancyValue)* MATRIX_END;
    // fancyValue: matrixValue | negatedValue | absentFeature | featureVariable;
    rule fancyMatrix() = "[" _ (matrixValue() / negatedValue() / absentFeature() / featureVariable())* "]" _
    // negatedValue: NEGATION matrixValue;
    rule negatedValue() = "!" matrixValue()
    // absentFeature: NULL name;
    rule absentFeature() = "*" name()
    // featureVariable: WORD_BOUNDARY name;
    rule featureVariable() = "$" name()

    // empty: NULL;
    rule empty() = "*" _
    // sylBoundary: SYLLABLE_BOUNDARY;
    // boundary: WORD_BOUNDARY;
    // betweenWords: BETWEEN_WORDS;
    // repeaterType: repeatRange | AT_LEAST_ONE | NULL | OPTIONAL;
    rule repeaterType() = repeatRange() / "+" _ / "*" _ / "?" _
    // repeatRange: NULL (NUMBER | (O_PAREN lowerBound? HYPHEN upperBound? C_PAREN));
    // lowerBound: NUMBER;
    // upperBound: NUMBER;
    rule repeatRange() = "*" (number() / ("(" _ number()? "-" number()? ")" _))
    // matrix: MATRIX_START matrixValue? (WHITESPACE matrixValue)* MATRIX_END;
    rule matrix() = "[" _ matrixValue()* "]" _
    // matrixValue: plusFeatureValue | featureValue;
    // plusFeatureValue: (AT_LEAST_ONE | HYPHEN) name;
    // featureValue: name;
    rule matrixValue() = ("+" / "-")? name()
    rule featureValue() = name()
    // text: (name | STR1 | STR) NEGATION?;
    rule text() = name() / sstr() "!"?
    // name:
    //     NAME |
    //     ELEMENT_DECL | CLASS_DECL | FEATURE_DECL | DIACRITIC_DECL | SYMBOL_DECL |
    //     SYLLABLE_DECL | CLEAR_SYLLABLES | EXPLICIT_SYLLABLES |
    //     DEROMANIZER | ROMANIZER | LITERAL |
    //     ALL_MATCHING | FIRST_MATCHING |
    //     LTR | RTL | PROPAGATE | BLOCK | CLEANUP |
    //     OFF | UNCHANGED;
    rule name() = sname()

    // CLASS_DECL: 'Class' | 'class';
    // FEATURE_DECL: 'Feature' | 'feature';
    // SYLLABLE_FEATURE: '(Syllable)' | '(syllable)';
    // DIACRITIC_DECL: 'Diacritic' | 'diacritic';
    // DIA_BEFORE: '(Before)' | '(before)';
    // DIA_FIRST: '(First)' | '(first)';
    // DIA_FLOATING: '(Floating)' | '(floating)';
    // SYMBOL_DECL: 'Symbol' | 'symbol';
    // SYLLABLE_DECL: 'Syllables' | 'syllables';
    // EXPLICIT_SYLLABLES: 'Explicit' | 'explicit';
    // CLEAR_SYLLABLES: 'Clear' | 'clear';
    // ANY_SYLLABLE: '<Syl>' | '<syl>';
    // DEROMANIZER: 'Deromanizer' | 'deromanizer';
    // ROMANIZER: 'Romanizer' | 'romanizer';
    // ALL_MATCHING: 'Then' | 'then';
    // FIRST_MATCHING: 'Else' | 'else';
    // LITERAL: 'Literal' | 'literal';
    // LTR: 'LTR' | 'Ltr' | 'ltr';
    // RTL: 'RTL' | 'Rtl' | 'rtl';
    // PROPAGATE: 'Propagate' | 'propagate';
    // CLEANUP: 'Cleanup' | 'cleanup';
    // BLOCK: 'Defer' | 'defer';
    // UNCHANGED: 'Unchanged' | 'unchanged';
    // OFF: 'Off' | 'off';
  }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(lsc::lsc_file(
            "
Feature soft
"
        ), Ok(vec![Stmt::FeatureDecl]));
    }
}
