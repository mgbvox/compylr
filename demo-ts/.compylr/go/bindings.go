package main

/*
#include <stdlib.h>
*/
import "C"

//export Call_collatzLength
func Call_collatzLength(n C.longlong) C.longlong {
	res := collatzLength(int64(n))
	return C.longlong(res)
}

//export Call_digitSum
func Call_digitSum(n C.longlong) C.longlong {
	res := digitSum(int64(n))
	return C.longlong(res)
}

//export Call_floorDivide
func Call_floorDivide(a C.longlong, b C.longlong) C.longlong {
	res := floorDivide(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_gcd
func Call_gcd(a C.longlong, b C.longlong) C.longlong {
	res := gcd(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_integerSqrt
func Call_integerSqrt(n C.longlong) C.longlong {
	res := integerSqrt(int64(n))
	return C.longlong(res)
}

//export Call_isPrime
func Call_isPrime(n C.longlong) C.longlong {
	res := isPrime(int64(n))
	if res {
		return 1
	} else {
		return 0
	}
}

//export Call_iterativeNotDivisible
func Call_iterativeNotDivisible(divisible C.longlong) C.longlong {
	res := iterativeNotDivisible(divisible != 0)
	if res {
		return 1
	} else {
		return 0
	}
}

//export Call_iterativeNthPrime
func Call_iterativeNthPrime(n C.longlong) C.longlong {
	res := iterativeNthPrime(int64(n))
	return C.longlong(res)
}

//export Call_larger
func Call_larger(a C.longlong, b C.longlong) C.longlong {
	res := larger(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_lcm
func Call_lcm(a C.longlong, b C.longlong) C.longlong {
	res := lcm(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_power
func Call_power(base C.longlong, exponent C.longlong) C.longlong {
	res := power(int64(base), int64(exponent))
	return C.longlong(res)
}

//export Call_recursiveIsPrime
func Call_recursiveIsPrime(n C.longlong) C.longlong {
	res := recursiveIsPrime(int64(n))
	if res {
		return 1
	} else {
		return 0
	}
}

//export Call_recursiveNextPrime
func Call_recursiveNextPrime(after C.longlong) C.longlong {
	res := recursiveNextPrime(int64(after))
	return C.longlong(res)
}

//export Call_recursiveNthPrime
func Call_recursiveNthPrime(n C.longlong) C.longlong {
	res := recursiveNthPrime(int64(n))
	return C.longlong(res)
}

//export Call_recursiveNthPrimeFrom
func Call_recursiveNthPrimeFrom(remaining C.longlong, current C.longlong) C.longlong {
	res := recursiveNthPrimeFrom(int64(remaining), int64(current))
	return C.longlong(res)
}

//export Call_remainder
func Call_remainder(a C.longlong, b C.longlong) C.longlong {
	res := remainder(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_smaller
func Call_smaller(a C.longlong, b C.longlong) C.longlong {
	res := smaller(int64(a), int64(b))
	return C.longlong(res)
}

//export Call_squareRoot
func Call_squareRoot(value C.longlong) C.longlong {
	res := squareRoot(int64(value))
	return C.longlong(res)
}

func main() {}
