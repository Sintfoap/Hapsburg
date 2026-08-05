" Vim/Neovim syntax file for Hapsburg (*.hb)
" A compiled language where everything is mandatory multiple inheritance.

if exists("b:current_syntax")
  finish
endif

syntax keyword hapsburgKeyword dynasty descends founder trait override abstract birth let return
syntax keyword hapsburgRepeat succession over as
syntax keyword hapsburgConditional claim contested
syntax keyword hapsburgBoolean true false
syntax keyword hapsburgSelf self
syntax keyword hapsburgOperatorWord and or

syntax keyword hapsburgType Integer String Bool List Void
syntax keyword hapsburgBuiltinFunc print abs assassinate
syntax match hapsburgNamespace "\<Habsburg\>"
syntax keyword hapsburgException InbreedingError

" A::B::C style dynasty/parent paths, e.g. after `dynasty` or `descends`.
syntax match hapsburgPath "\<[A-Za-z_][A-Za-z0-9_]*\(::[A-Za-z_][A-Za-z0-9_]*\)\+"

syntax match hapsburgNumber "\<[0-9]\+\>"

syntax match hapsburgEscape contained "\\[\"\\ntr]"
syntax region hapsburgString start=+"+ skip=+\\\\\|\\"+ end=+"+ contains=hapsburgEscape

syntax match hapsburgOperatorSym "->\|::\|==\|!=\|<=\|>=\|[+\-*/%<>=|]"

syntax match hapsburgComment "//.*$" contains=@Spell

highlight default link hapsburgKeyword       Keyword
highlight default link hapsburgRepeat        Repeat
highlight default link hapsburgConditional   Conditional
highlight default link hapsburgBoolean       Boolean
highlight default link hapsburgSelf          Special
highlight default link hapsburgOperatorWord  Keyword
highlight default link hapsburgType          Type
highlight default link hapsburgBuiltinFunc   Function
highlight default link hapsburgNamespace     Structure
highlight default link hapsburgException     Exception
highlight default link hapsburgPath          Type
highlight default link hapsburgNumber        Number
highlight default link hapsburgString        String
highlight default link hapsburgEscape        SpecialChar
highlight default link hapsburgOperatorSym   Operator
highlight default link hapsburgComment       Comment

let b:current_syntax = "hapsburg"
