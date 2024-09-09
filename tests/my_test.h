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

MY_LIB_API int add(int a, int b); // this is a comment.

MY_LIB_API void this_will_crash();

typedef int my_int32;

typedef struct myStruct {
  int badName_1;
  char BAD_NAME_2;
} myStruct;

MY_LIB_API void change_struct(int a, char b, myStruct *c);

void badFunc(myStruct a) { a.BAD_NAME_2 = 1; }

#endif