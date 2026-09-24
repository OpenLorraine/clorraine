" Clorraine syntax highlighting for Vim.
"
" Clorraine looks like C, so this file reuses Vim's C syntax for braces,
" strings, numbers, operators, and comments, then adds the words Clorraine has
" that C does not (func, module, let, the fixed-width integer types, and the
" standard-library handle types).
"
" Open a file with it through demo/show-source.sh, or directly with:
"
"   vim -R -c 'source demo/clorraine.vim' FILE.clo

if exists("b:current_syntax")
  finish
endif

" Bring in the C rules first; drop the marker so this file can take over.
runtime! syntax/c.vim
unlet! b:current_syntax

" Declarations and control words.
syntax keyword cloKeyword
      \ module import export func let take borrow handle deleter
      \ ffi intrinsic unsafe raw volatile enum struct const static
      \ comptime defer
      \ if else while for switch case default break continue return
      \ true false null

" Scalar and view types.
syntax keyword cloType
      \ i8 i16 i32 i64 u8 u16 u32 u64 usize isize f32 f64 bool void
      \ str cstring

" Owned standard-library handle types.
syntax keyword cloHandleType String File Buffer HandlerToken

" The `$` of a string template, so the interpolation marker stands out.
syntax match cloTemplate "\$\ze\""

" A name used as a call: a word immediately followed by `(`.
syntax match cloFunction "\<\h[A-Za-z0-9_]*\>\ze\s*("

highlight default link cloKeyword Keyword
highlight default link cloType Type
highlight default link cloHandleType Structure
highlight default link cloTemplate SpecialChar
highlight default link cloFunction Function

let b:current_syntax = "clorraine"
