/* Test-only worker fixture. Never included in the installed application. */
#include <windows.h>
#include <stdio.h>
int main(void) {
  fputs("{\"text\":\"\",\"error\":\"\"}\n", stdout);
  fflush(stdout);
  Sleep(INFINITE);
  return 0;
}
