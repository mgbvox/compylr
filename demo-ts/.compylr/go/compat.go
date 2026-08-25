package main

import (
	"unicode/utf8"
)

// GoFloorDiv computes floor division matching Python/TS integer division.
func GoFloorDiv(a, b int64) int64 {
	if b == 0 {
		panic("division by zero")
	}
	res := a / b
	rem := a % b
	if (a < 0) != (b < 0) && rem != 0 {
		res--
	}
	return res
}

// GoRem computes remainder with sign of divisor (Python/TS convention).
func GoRem(a, b int64) int64 {
	if b == 0 {
		panic("division by zero")
	}
	rem := a % b
	if (rem < 0 && b > 0) || (rem > 0 && b < 0) {
		rem += b
	}
	return rem
}

// GoSubscript resolves positive or negative-from-end slice indexing.
func GoSubscript[T any](slice []T, index int64) T {
	n := int64(len(slice))
	if index < 0 {
		index += n
	}
	return slice[index]
}

// GoMapGet returns value from map or zero value.
func GoMapGet[K comparable, V any](m map[K]V, key K) V {
	return m[key]
}

// GoKeys returns slice of keys from a map.
func GoKeys[K comparable, V any](m map[K]V) []K {
	keys := make([]K, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return keys
}

// GoSetKeys returns slice of keys from a set.
func GoSetKeys[K comparable](s map[K]struct{}) []K {
	keys := make([]K, 0, len(s))
	for k := range s {
		keys = append(keys, k)
	}
	return keys
}

// GoRuneLen returns the number of UTF-8 code points in a string.
func GoRuneLen(s string) int64 {
	return int64(utf8.RuneCountInString(s))
}
