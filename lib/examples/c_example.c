#include <stdio.h>
#include <stdlib.h>
#include "../reticula_ffi.h"

int main() {
    printf("=== Reticula FFI C Example ===\n");
    
    // Test version function
    printf("Library version: %s\n", get_version());
    
    // Test simple math function
    int result = multiply_by_two(21);
    printf("multiply_by_two(21) = %d\n", result);
    
    // Test calculator
    printf("\n--- Calculator Test ---\n");
    Calculator* calc = calculator_new();
    if (calc) {
        printf("Initial value: %.2f\n", calculator_get_value(calc));
        
        calculator_add(calc, 10.5);
        printf("After adding 10.5: %.2f\n", calculator_get_value(calc));
        
        calculator_add(calc, -3.2);
        printf("After adding -3.2: %.2f\n", calculator_get_value(calc));
        
        calculator_free(calc);
        printf("Calculator freed\n");
    }
    
    // Test string processing
    printf("\n--- String Processing Test ---\n");
    const char* input = "hello world";
    char* processed = process_string(input);
    if (processed) {
        printf("Input: %s\n", input);
        printf("Output: %s\n", processed);
        free_string(processed);
    }
    
    printf("\nAll tests completed!\n");
    return 0;
}