//! Runtime helper functions embedded into generated Go packages.

pub const GO_COMPAT_SOURCE: &str = r#"package main

import (
	"errors"
	"unicode/utf8"
)

var (
	ErrDivisionByZero = errors.New("division by zero")
	ErrIndexOutOfRange = errors.New("index out of range")
	ErrKeyNotFound     = errors.New("key not found")
)

// GoFloorDiv computes floor division matching Python/TS integer division.
func GoFloorDiv(a, b int64) (int64, error) {
	if b == 0 {
		return 0, ErrDivisionByZero
	}
	res := a / b
	rem := a % b
	if (a < 0) != (b < 0) && rem != 0 {
		res--
	}
	return res, nil
}

// GoRem computes remainder with sign of divisor (Python/TS convention).
func GoRem(a, b int64) (int64, error) {
	if b == 0 {
		return 0, ErrDivisionByZero
	}
	rem := a % b
	if (rem < 0 && b > 0) || (rem > 0 && b < 0) {
		rem += b
	}
	return rem, nil
}

// GoSubscript resolves positive or negative-from-end slice indexing.
func GoSubscript[T any](slice []T, index int64) (T, error) {
	var zero T
	n := int64(len(slice))
	if index < 0 {
		index += n
	}
	if index < 0 || index >= n {
		return zero, ErrIndexOutOfRange
	}
	return slice[index], nil
}

// GoRuneLen returns the number of UTF-8 code points in a string.
func GoRuneLen(s string) int64 {
	return int64(utf8.RuneCountInString(s))
}
"#;
