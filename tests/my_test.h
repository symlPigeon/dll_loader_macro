#ifndef MY_TEST_H

#ifdef _WIN32
#ifdef MY_LIB_EXPORTS
#define MY_LIB_API __declspec(dllexport)
#else
#define MY_LIB_API __declspec(dllimport)
#endif
#else
#define MY_LIB_API
#endif

#define ANSWER 42

MY_LIB_API int add(int a, int b);

MY_LIB_API void this_will_crash();

typedef struct myStruct {
  int a;
  char b;
} myStruct;

MY_LIB_API void change_struct(int a, char b, myStruct *c);

#endif