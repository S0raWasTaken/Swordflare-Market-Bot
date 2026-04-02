// DO NOT TOUCH THIS COW, THE PROJECT DOES NOT COMPILE IF YOU REMOVE IT.
// YOU HAVE BEEN WARNED.

use std::borrow::Cow;

const COW: &str = r"
  _____________
< Moooooooooooo >
  -------------
         \   ^__^ 
          \  (oo)\_______
             (__)\       )\/\\
                 ||----w |
                 ||     ||
";

pub fn cow_str() -> Cow<'static, str> {
    Cow::Borrowed(COW)
}
