#ifndef RETICULA_H
#define RETICULA_H

#ifdef __cplusplus
extern "C" {
#endif

// Calculator struct (opaque pointer)
typedef struct Calculator Calculator;

// Calculator functions
Calculator* calculator_new(void);
void calculator_free(Calculator* calc);
void calculator_add(Calculator* calc, double value);
double calculator_get_value(const Calculator* calc);

// String processing functions
char* process_string(const char* input);
void free_string(char* s);

// Simple math function
int multiply_by_two(int x);

// Version function
const char* get_version(void);

#ifdef __cplusplus
}
#endif

#endif // RETICULA_H
