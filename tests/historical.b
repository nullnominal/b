main() {
   extrn printf;
   auto a;
   a = 1;
   a =+ 10;
   a =- 1;
   a =% 2;
   a =* 5;
   /* a = 5 */
   
   printf("%i", a);
   a =| (1 << 5);
   a =& (1 << 5);
   a =<< 1;
   a =>> 2;
   /* a = 2**3 */
   printf("%i", a);
}
