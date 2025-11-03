// FIXME 7. feladat
use std::collections::VecDeque;



const ESCAPE_STARTER: &str = "#{";
const ESCAPE_ENDER: char = '}';


pub struct Settings {
    pub active_tags: Vec<String>
}


pub enum ParseState {
    NormalText,
    MaybeEscape,
    Command
}


struct State {
    active_tags: Vec<String>
}


pub struct TextPos {
    pub line: usize,
    pub col: usize
}

impl TextPos {
    fn new() -> Self {
        Self {
            line: 1,
            col: 1
        }
    }

    fn next_char(&mut self) {
        self.col += 1;
    }

    fn line_break(&mut self) {
        self.col = 1;
        self.line += 1;
    }
}

impl std::fmt::Display for TextPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, col {}", self.line, self.col)?;
        Result::Ok(())
    }
}


pub struct Processor<'a> {
    pub settings: &'a Settings,
    pub parse_state: ParseState,
    pub text_pos: TextPos,
    state: State,
    pub limbo: VecDeque<char>,
    pub output: VecDeque<char>
}

impl<'a> Processor<'a> {

    pub fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            parse_state: ParseState::NormalText,
            text_pos: TextPos::new(),
            state: State {
                active_tags: Vec::<String>::new()
            },
            limbo: VecDeque::<char>::new(),
            output: VecDeque::<char>::new(),
        }
    }

    pub fn process_text(text: &str, settings: &'a Settings) -> VecDeque<char> {
        let mut proc = Processor::new(settings);

        proc.process_many(text.chars());
        proc.know_nothing_but_hunger();

        proc.output
    }


    pub fn process_char(&mut self, ch: char) {
        match self.parse_state {
            ParseState::NormalText => {
                if ch == ESCAPE_STARTER.chars().nth(0).expect("`ESCAPE_STARTER` should be non-empty") {
                    self.limbo.push_back(ch);
                    self.parse_state = ParseState::MaybeEscape;
                } else {
                    if self.text_conditions_met() {
                        self.output.push_back(ch);
                    }
                }
            },
            ParseState::MaybeEscape => {
                self.limbo.push_back(ch);
                if ch == ESCAPE_STARTER.chars().nth(self.limbo.len() - 1).expect("`self.limbo` should never have more chars in it than `ESCAPE_STARTER` when maybe reading an escape sequence") {
                    if self.limbo.len() == ESCAPE_STARTER.len() {
                        self.limbo.clear();
                        self.parse_state = ParseState::Command;
                    }
                } else {
                    self.know_nothing_but_hunger();
                    self.parse_state = ParseState::NormalText;
                }
            },
            ParseState::Command => {
                if ch == ESCAPE_ENDER {
                    let command: String = self.limbo.drain(..).collect();
                    self.do_command(&command);
                    self.parse_state = ParseState::NormalText;
                } else {
                    self.limbo.push_back(ch);
                }
            }
        }

        if ch == '\n' {
            self.text_pos.line_break();
        } else {
            self.text_pos.next_char();
        }
    }

    pub fn process_many(&mut self, iter: impl Iterator<Item = char>) {
        for ch in iter {
            self.process_char(ch);
        }
    }


    fn know_nothing_but_hunger(&mut self) {
        self.output.extend(self.limbo.drain(..));
    }

    fn text_conditions_met(&self) -> bool {
        self.state.active_tags.iter()
            .all(|tag: &String| self.settings.active_tags.contains(tag))
    }

    fn do_command(&mut self, command: &str) {
        let what: &str;
        let args: Option<Vec<&str>>;

        if let Some((name, arg_list)) = command.split_once('(') {
            what = name;

            let arg_parts = arg_list.rsplit_once(')').expect(&format!("command argument list should be closed with ')' ({})", self.text_pos));
            assert_eq!(arg_parts.1.len(), 0, "there shouldn't be more chars after a command's argument list has been closed ({})", self.text_pos);

            args = Some(arg_parts.0.split(',')
                .map(|arg| arg.trim())
                .collect());
        } else {
            assert!(!command.contains(')'), "command without argument list shouldn't contain ')' ({})", self.text_pos);

            what = command;
            args = None;
        }

        match what {
            "nop" => { /* do nothing */ },
            "if" => {
                self.state.active_tags.extend(args.expect(&format!("\"if\" should have arguments ({})", self.text_pos)).iter().map(|x| x.to_string()));
            },
            "end" => {
                if let Some(what_ends) = args {
                    for this_ends in what_ends {
                        let index: usize = self.state.active_tags.iter()
                            .enumerate()
                            .rev()
                            .find(|x| x.1 == this_ends)
                            .expect(&format!("\"end\"'s arguments should all be things that can be ended ({})", self.text_pos))
                            .0;
                        assert_eq!(self.state.active_tags.remove(index), this_ends);
                    }
                } else {
                    self.state.active_tags.pop().expect(&format!("\"end\" can only be used if there is something to actually end ({})", self.text_pos));
                }
            },
            _ => panic!("invalid command:  \"{}\" ({})", what, self.text_pos)
        }
    }

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing() {
        let input = "# This text contains no escapes.";

        let expected = input;
        let actual = Processor::process_text(input, &Settings { active_tags: vec![/* empty */] }).iter().collect::<String>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_nop() {
        let input = "# This text h#{nop}as a nop in it.";

        let expected = "# This text has a nop in it.";
        let actual = Processor::process_text(input, &Settings { active_tags: vec![/* empty */] }).iter().collect::<String>();
        assert_eq!(expected, actual);
    }

    #[test]
    #[should_panic]
    fn test_invalid_command() {
        let input = "# This text h#{iNvAlId}as an INVALID COMMAND!! in it.";
        let _ = Processor::process_text(input, &Settings { active_tags: vec![/* empty */] });
    }

    #[test]
    fn test_ab() {
        let input = "# Melyek igazak #{if(A)}a sor (queue)#{end}#{if(B)}a verem (stack)#{end(B)} adatszerkezetre?";

        let expected_a = "# Melyek igazak a sor (queue) adatszerkezetre?";
        let actual_a = Processor::process_text(input, &Settings { active_tags: vec![ "A".to_string() ] }).iter().collect::<String>();
        assert_eq!(expected_a, actual_a);

        let expected_b = "# Melyek igazak a verem (stack) adatszerkezetre?";
        let actual_b = Processor::process_text(input, &Settings { active_tags: vec![ "B".to_string() ] }).iter().collect::<String>();
        assert_eq!(expected_b, actual_b);
    }

    #[test]
    fn test_python1() {
        let input = "def sum_to(n):\n    pass # TODO#{if(solved)}\n    if n <= 0:\n        return 0\n    else:\n        return n + sum_to(n - 1)\n#{end(solved)}\n# blah blah";

        let expected_a = "def sum_to(n):\n    pass # TODO\n    if n <= 0:\n        return 0\n    else:\n        return n + sum_to(n - 1)\n\n# blah blah";
        let actual_a = Processor::process_text(input, &Settings { active_tags: vec![ "solved".to_string() ] }).iter().collect::<String>();
        assert_eq!(expected_a, actual_a);

        let expected_b = "def sum_to(n):\n    pass # TODO\n# blah blah";
        let actual_b = Processor::process_text(input, &Settings { active_tags: vec![/* empty */] }).iter().collect::<String>();
        assert_eq!(expected_b, actual_b);
    }

    #[test]
    fn test_python2() {
        // FIXME (probably something to do with nested tags?)
        let input = r##"
# ~ 7. feladat ~
# Visszaadja, hány darab #{if(A)}páratlan#{end}#{if(B)}páros#{end} szám van a listában.#{if(A)}
def number_of_odd(list):
    pass # TODO#{if(solved)}
    if len(list) == 0:
        return 0
    else:
        if list[0] % 2 == 1:
            return 1 + number_of_odd(list[1:])
        else:
            return number_of_odd(list[1:])
#{end(solved)}

print('\n' + banner("7.) number_of_odd"))
test('number_of_odd([])', 0)
test('number_of_odd([1])', 1)
test('number_of_odd([2])', 0)
test('number_of_odd([5, 6, 7])', 2)
test('number_of_odd([10, 14, 15])', 1)#{end}#{if(B)}
def number_of_even(list):
    pass # TODO#{if(solved)}
    if len(list) == 0:
        return 0
    else:
        if list[0] % 2 == 0:
            return 1 + number_of_even(list[1:])
        else:
            return number_of_even(list[1:])
#{end(solved)}

print('\n' + banner("7.) number_of_even"))
test('number_of_even([])', 0)
test('number_of_even([1])', 0)
test('number_of_even([2])', 1)
test('number_of_even([5, 6, 7])', 1)
test('number_of_even([10, 14, 15])', 2)#{end}"##;

        let expected_a = r##"
# ~ 7. feladat ~
# Visszaadja, hány darab páratlan szám van a listában.
def number_of_odd(list):
    pass # TODO

print('\n' + banner("7.) number_of_odd"))
test('number_of_odd([])', 0)
test('number_of_odd([1])', 1)
test('number_of_odd([2])', 0)
test('number_of_odd([5, 6, 7])', 2)
test('number_of_odd([10, 14, 15])', 1)"##;
        let actual_a: String = Processor::process_text(input, &Settings { active_tags: vec![ "A".to_string() ] }).iter().collect();
        assert_eq!(expected_a, actual_a);

        let expected_b = r##"
# ~ 7. feladat ~
# Visszaadja, hány darab páros szám van a listában.
def number_of_even(list):
    pass # TODO

print('\n' + banner("7.) number_of_even"))
test('number_of_even([])', 0)
test('number_of_even([1])', 0)
test('number_of_even([2])', 1)
test('number_of_even([5, 6, 7])', 1)
test('number_of_even([10, 14, 15])', 2)"##;
        let actual_b: String = Processor::process_text(input, &Settings { active_tags: vec![ "B".to_string() ] }).iter().collect();
        assert_eq!(expected_b, actual_b);
    }
}
