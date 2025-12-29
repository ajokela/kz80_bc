#!/bin/bash
# Comprehensive math tests for bc80
# Runs expressions through the Z80 emulator and verifies results

BC80="./target/release/bc80"
EMULATOR="../emulator/retroshield"
TMPBC="/tmp/bc80_test.bc"
TMPBIN="/tmp/bc80_test.bin"

PASS=0
FAIL=0

test_expr() {
    local expr="$1"
    local expected="$2"

    echo "$expr" > "$TMPBC"
    $BC80 "$TMPBC" --rom "$TMPBIN" 2>/dev/null
    local result=$(timeout 2 $EMULATOR -c 2000000 "$TMPBIN" 2>/dev/null | tr -d '\n\r')

    if [ "$result" = "$expected" ]; then
        echo "PASS: $expr = $expected"
        PASS=$((PASS + 1))
    else
        echo "FAIL: $expr expected '$expected' but got '$result'"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Basic Integer Operations ==="
test_expr "0" "0"
test_expr "1" "1"
test_expr "42" "42"
test_expr "999" "999"
test_expr "12345" "12345"

echo ""
echo "=== Addition ==="
test_expr "1+1" "2"
test_expr "0+0" "0"
test_expr "5+3" "8"
test_expr "10+20" "30"
test_expr "99+1" "100"
test_expr "100+200+300" "600"

echo ""
echo "=== Subtraction ==="
test_expr "5-3" "2"
test_expr "3-5" "-2"
test_expr "0-5" "-5"
test_expr "10-10" "0"
test_expr "100-1" "99"
test_expr "1000-999" "1"

echo ""
echo "=== Mixed Add/Sub ==="
test_expr "5+3-2" "6"
test_expr "10-5+3" "8"
test_expr "1-10+5" "-4"
test_expr "0-9+5" "-4"
test_expr "100-50+25" "75"
test_expr "10+5-15" "0"

echo ""
echo "=== Multiplication ==="
test_expr "2*3" "6"
test_expr "0*5" "0"
test_expr "1*1" "1"
test_expr "10*10" "100"
test_expr "12*12" "144"
test_expr "7*8" "56"

echo ""
echo "=== Division ==="
test_expr "8/2" "4"
test_expr "10/2" "5"
test_expr "9/3" "3"
test_expr "100/10" "10"
test_expr "7/2" "3"
test_expr "1/2" "0"

echo ""
echo "=== Division with Scale ==="
test_expr "scale=1; 7/2" "3.5"
test_expr "scale=2; 1/4" ".25"
test_expr "scale=3; 7/2" "3.500"
test_expr "scale=3; 22/7" "3.142"
test_expr "scale=5; 22/7" "3.14285"
test_expr "scale=10; 1/7" ".1428571428"

echo ""
echo "=== Decimal Numbers ==="
test_expr "0.5" ".5"
test_expr ".001" ".001"
test_expr "3.14159" "3.14159"
test_expr "1.5+1.5" "3.0"
test_expr "2.5-1.0" "1.5"

echo ""
echo "=== Negative Numbers ==="
test_expr "0-1" "-1"
test_expr "0-100" "-100"
test_expr "1-2" "-1"
test_expr "5-10" "-5"

echo ""
echo "=== Order of Operations ==="
test_expr "2+3*4" "14"
test_expr "10-2*3" "4"
test_expr "2*3+4*5" "26"
test_expr "100/10+5" "15"

echo ""
echo "=== Parentheses ==="
test_expr "(2+3)*4" "20"
test_expr "2*(3+4)" "14"
test_expr "(10-5)*2" "10"
test_expr "((1+2)*3)" "9"

echo ""
echo "=== Large Numbers ==="
test_expr "9999+1" "10000"
test_expr "10000-1" "9999"
test_expr "1000*1000" "1000000"
test_expr "999999+1" "1000000"

echo ""
echo "=== Variables ==="
test_expr "a=5; a" "5"
test_expr "a=5; a+3" "8"
test_expr "a=10; b=20; a+b" "30"
test_expr "x=100; x-50" "50"

echo ""
echo "=== Edge Cases ==="
test_expr "0+0" "0"
test_expr "0-0" "0"
test_expr "0*0" "0"
test_expr "0*100" "0"
test_expr "100*0" "0"
test_expr "1-1" "0"
test_expr "5-5" "0"

echo ""
echo "=== Negative with Negative ==="
test_expr "0-3-2" "-5"
test_expr "0-5-5" "-10"
test_expr "10-20-30" "-40"

echo ""
echo "=== Complex Expressions ==="
test_expr "2*3+4*5-6" "20"
test_expr "(1+2)*(3+4)" "21"
test_expr "100/10/2" "5"
test_expr "2*2*2*2" "16"

echo ""
echo "=== Decimal Arithmetic ==="
test_expr "scale=2; 1.5+2.5" "4.0"
test_expr "scale=2; 5.0-2.5" "2.5"
test_expr "scale=2; 2.5*2" "5.0"
test_expr "scale=2; 10.0/4" "2.50"
test_expr "scale=3; 1.5*2.5" "3.75"
test_expr "scale=2; 7.5/2.5" "3.00"

echo ""
echo "=== Summary ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Total:  $((PASS + FAIL))"

if [ $FAIL -gt 0 ]; then
    exit 1
fi
