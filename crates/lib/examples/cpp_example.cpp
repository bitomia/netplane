#include <iostream>
#include <string>
#include "../reticula_ffi.h"

int main() {
    std::cout << "=== Reticula FFI C++ Example ===" << std::endl;
    
    // Test version function
    std::cout << "Library version: " << get_version() << std::endl;
    
    // Test simple math function
    int result = multiply_by_two(21);
    std::cout << "multiply_by_two(21) = " << result << std::endl;
    
    // Test calculator
    std::cout << "\n--- Calculator Test ---" << std::endl;
    Calculator* calc = calculator_new();
    if (calc) {
        std::cout << "Initial value: " << calculator_get_value(calc) << std::endl;
        
        calculator_add(calc, 10.5);
        std::cout << "After adding 10.5: " << calculator_get_value(calc) << std::endl;
        
        calculator_add(calc, -3.2);
        std::cout << "After adding -3.2: " << calculator_get_value(calc) << std::endl;
        
        calculator_free(calc);
        std::cout << "Calculator freed" << std::endl;
    }
    
    // Test string processing
    std::cout << "\n--- String Processing Test ---" << std::endl;
    const char* input = "hello world";
    char* processed = process_string(input);
    if (processed) {
        std::cout << "Input: " << input << std::endl;
        std::cout << "Output: " << processed << std::endl;
        free_string(processed);
    }
    
    std::cout << "\nAll tests completed!" << std::endl;
    return 0;
}