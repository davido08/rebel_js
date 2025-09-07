/*This project is an open source JavaScript/HTML/CSS compressor
meant to drastically reduce the size of many websites.

It allows develoopers to write verbose, concise and clear code in developpment,
and get short and optimized code in production effortlessly and super quickly.

NOTE: This program is still in Beta and is open source, so if you notice any bugs, possible optimizations
or parts of the code that could be improved, don't hesitate to hit me up on my GitHub: --insert github link--

The logic of the code is moderately complex, but i will try my best to explain through the code and 
more in depth in the documentation available on the --insert file name-- file.


- David Firlotte, 2025

*/

mod characters;
mod compression;
mod code_analysis;
use core::panic;
use std::collections::binary_heap::Iter;
use std::collections::HashMap;
use std::fmt::Error;
use std::process::exit;
use std::thread::current;
use tokio::{fs::File, io::AsyncReadExt};
use tokio::sync::mpsc;
use std::{io::Read, str};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::rc::Rc;
use std::cell::RefCell;
use regex::Regex;
use std::env::{self, set_current_dir};

use crate::code_analysis::{GoingThrough, PreviousBufferState};


#[derive(Clone, PartialEq, Eq, Hash)]
//Establishing constans in order for the code to be more understandable 

//Error declarations 

struct InvalidSyntaxError;

impl fmt::Display for InvalidSyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Code given as an input seems to contain a sytax error.
        Please make sure that the code served as an input is 100% operational.
        If this seems like a mistake, note that this program is currently under development and we would like to get your feedback as soon as possible.
        You can report bugs or unusual behaviour on our -----INSERT CONTACT PAGE HERE------")
    }
}

fn value_name_is_valid(a: u8, b: u8) -> bool {
    match a {
        0..=47 | 59..=64 | 91..=96 | characters::LEFT_CURLY..=127 => {
            match b {
                0..=47 | 59..=64 | 91..=96 | characters::LEFT_CURLY..=127 => {
                    true
                }
                _ => {
                    false
                }
            }
        }
        _ => {
            false
        }
    }
}

fn find_value_to_replace(code_analysis_object: &mut code_analysis::CodeAnalysisObject, q_char: &mut u8, j: &mut usize, changed_values: &HashMap<[u8;2], u8>) {
    let mut jdx = *j;
    let mut q_char_dx= *q_char;
    match q_char_dx {
        39 | 34 => {
            if code_analysis_object.current_scope.characters[jdx] == q_char_dx {
                q_char_dx=0;
            }
        }
        96 => {
            match code_analysis_object.current_scope.characters[jdx] {
                96=> {
                    q_char_dx=0;
                }
                36 if code_analysis_object.current_scope.characters[jdx+1] == characters::LEFT_CURLY=> {
                        jdx+=2;
                        while code_analysis_object.current_scope.characters[jdx] != characters::RIGHT_CURLY {
                            let mut new_q_char: u8 = 0;
                            find_value_to_replace(&mut new_q_char, &mut jdx,  changed_values, code_analysis_object.current_scope);
                            jdx+=1;
                        }
                        jdx+=1;
                }
                _=> {}
            }
        
        } 
        _ => {
            for (old_val, new_val) in changed_values {
                if code_analysis_object.current_scope.characters[jdx] == old_val[0] && 
                   code_analysis_object.current_scope.characters[jdx+1] == old_val[1] &&
                   value_name_is_valid(code_analysis_object.current_scope.characters[jdx-1], code_analysis_object.current_scope.characters[jdx+2]) 
                {
                    code_analysis_object.current_scope.characters.remove(jdx+1);
                    code_analysis_object.current_scope.characters[jdx] = *new_val;

                }
            
            }
        }

    }

    *j=jdx;
    *q_char=q_char_dx;

}



fn find_quote_chars(
    quote_char: &mut u8, 
    going_through: &mut code_analysis::GoingThrough,
    code_analysis_object: &mut code_analysis::CodeAnalysisObject,
) {
    
    let byte = code_analysis_object.buffer_ref[*code_analysis_object.i_buffer];
    code_analysis_object.add_one_byte_char(byte);
    if byte == *quote_char {
        *quote_char = 0;
       
    } else if *quote_char == 96  {

        if byte == 36 && find_if_corresponds(code_analysis_object.buffer_ref,  code_analysis_object.i_buffer, vec![characters::LEFT_CURLY]) {
            *code_analysis_object.i_buffer+=1;
            
            look_for_values_js( true,  going_through, code_analysis_object );
        }

    } 
}

fn look_for_values_js( 
    inside_concat_string: bool,
    going_through: &mut code_analysis::GoingThrough,
    code_analysis_object: &mut code_analysis::CodeAnalysisObject,


) { 
    let mut idx =*code_analysis_object.i_buffer;
    let mut quote_char: u8 = 0;

    while idx < code_analysis_object.buffer_ref.len() {
        let byte = code_analysis_object.buffer_ref[*code_analysis_object.i_buffer];
        if quote_char != 0 {
            find_quote_chars(code_analysis_object.buffer_ref, code_analysis_object.current_scope, code_analysis_object.i_buffer, &mut quote_char, code_analysis_object.previous_buffer_state, going_through);
        } else {
            match byte {
                10 => {}

            }
        }
    }


}




fn find_value(
    code_analysis_object: &mut code_analysis::CodeAnalysisObject,
    all_values_count: &mut HashMap<code_analysis::NewValueName, u16>
) {

    let mut idx = *code_analysis_object.i_buffer;
    let startindex = idx;
        let mut scope = Rc::new(RefCell::new(code_analysis_object.current_scope.clone()));
        let mut j = idx+1;



        while j< code_analysis_object.buffer_ref.len() {
            match code_analysis_object.buffer_ref[j] {
                0..=47 | 58..=64 | 91 | 93 | 94 | characters::LEFT_CURLY..=128 => {
                    break;
                }
                _=> {
                    j+=1;
                }
            }
        }

        let value_name = code_analysis_object.buffer_ref[startindex..j].to_vec();

        loop {
            let mut scope_ref = scope.borrow_mut();
            
                for value in scope_ref.values.iter_mut() {
                    if value.value_old_name == value_name {

                        //Put the value in the blocked values of the potential values
                        for potential_value in &mut code_analysis_object.current_scope.potential_values_called {
                            if potential_value.name == value_name {
                                match value.value_new_name {
                                    code_analysis::NewValueName::OneChar(byte) => {
                                        let index = (byte - 65) as usize; // Convert A-Z to 0-25
                                        if index < 58 {
                                            potential_value.blocked_values[index] = true;
                                        }
                                    }
                                    code_analysis::NewValueName::TwoChar(_) => {
                                        // Handle two-char case if needed
                                        // For now, skip as blocked_values only has 58 slots
                                    }
                                }
                            }
                        }


                        let mut contained = false;
                        for val in &mut code_analysis_object.current_scope.used_values {
                            if val.value_new_name == value.value_new_name {
                                val.last_usage_index = idx;
                                val.amount_occurences+=1;
                                contained=true;
                            }
                        }

                        
                        if !contained {
                            code_analysis_object.current_scope.used_values.push(value.clone());
                            code_analysis_object.current_scope.children_used_values.push(value.clone());
                        }
                        value.amount_occurences +=1;
                        all_values_count.insert(
                            value.value_new_name.clone(),
                            *all_values_count.get(&mut value.value_new_name).unwrap() + 1,
                        );
                        match value.value_new_name {
                            code_analysis::NewValueName::OneChar(x)=> {code_analysis_object.add_one_byte_char(x);}
                            code_analysis::NewValueName::TwoChar(x) => {code_analysis_object.current_scope.characters.append(&mut x.to_vec());}
                        }

                        code_analysis_object.add_one_byte_char(code_analysis_object.buffer_ref[j]);
                        *code_analysis_object.i_buffer = j-1;
                        return;

                    }
                }
                            
                       
            // Traverse up the parent chain
            let next_scope = match &scope_ref.parent_scope {
                Some(parent) => Some(parent.clone()),
                None => break,
            };
            drop(scope_ref); // End the borrow before reassigning
            scope = next_scope.unwrap();
        }
        


        match value_name {
            //console | window | document | fetch |
            [99,111,110,115,111,108,101] | [119,105,110,100,111,119] | [100,111,99,117,109,101,110,116] | [102,101,116,99,104] => {
                
            } _ => {
                if !code_analysis_object.current_scope.potential_values_called.iter().any(|val| val.name == value_name) {
                    let is_function = {
                        let k = j;
                        while k < code_analysis_object.buffer_ref.len() {
                            match code_analysis_object.buffer_ref[k] {
                                32 => k+=1,
                                40 => true, 
                                _ => false
                                
                            }
                            false
                    }
                    
                };
                code_analysis_object.current_scope.potential_values_called.push(PotentialValue::new(value_name, is_function, code_analysis_object.current_scope.children_used_values));
            }
        }
    
    }
}


fn assign_end_scope_char(code_analysis_object: &mut code_analysis::CodeAnalysisObject, previous_buffer_state: &mut code_analysis::PreviousBufferState, end_scope_char: code_analysis::EndOfScopeChar) {
    
    code_analysis_object.current_scope.beginning_chars = std::mem::take(&mut code_analysis_object.current_scope.characters); 
    code_analysis_object.current_scope.end_scope_char = end_scope_char;
    *code_analysis_object.previous_buffer_state = code_analysis::PreviousBufferState::Code;
}

fn scope_end(code_analysis_object: &mut code_analysis::CodeAnalysisObject, all_values_count: &mut HashMap<code_analysis::NewValueName, u16>, previous_buffer_state: &mut code_analysis::PreviousBufferState)-> io::Result<()> {

    code_analysis_object.add_one_byte_char(characters::RIGHT_CURLY);
    if let Some(scope) = code_analysis_object.current_scope.parent_scope.clone() {
        let mut changed_values: HashMap<[u8;2], u8> = HashMap::new();
        
        for value in &mut code_analysis_object.current_scope.two_byte_name_values {
            if let code_analysis::NewValueName::TwoChar(two_char) =value.value_new_name {
            let mut changed_value : Option<u8> = None;
            //List of changed values with their old two byte name and their new one byte name
            let changed_values: Vec<([u8;2], u8)> = Vec::new();

            for index in 0..52 {
                /*If the last time the value was called is before the assignment
                 of the value we want to replace, then we can go ahead and replace it
                 without affecting the functionnality of the code. */
                 if let Some(last_usage) = code_analysis_object.current_scope.children_used_values.new_value_last_position[index] {
                    if last_usage < value.declaration_index {

                        let byte = match index {
                            0..=25 => index + 65,
                            26..=51 => index + 71,
                        } as u8;

                        changed_values.push((two_char, byte))
                    }
                }
                    
                }
            }
        }

        
        //Creating a new vector which we will add the parent scope characters we are going back to.
        let new_characters: Vec<u8> = Vec::with_capacity(code_analysis_object.current_scope.characters.len() - changed_values.len()*2);

        //Adding a quote char so this way we don't accidentally replace something inside quotes
        let mut quote_char: u8 = 0;
        //Concatenation will increase by 1 if either 
        let mut concatenation: u8 = 0;

        let mut j: usize = 0;

        let mut next_valid=true;

        let len =  code_analysis_object.current_scope.characters.len();

        while j < len {
            let mut byte = code_analysis_object.current_scope.characters[j];
            if quote_char != 0 {
                if byte == quote_char {
                    quote_char = 0;
                }
            }
            //If is inside a quoted sentence with a grave accent
            else if concatenation & 1 == 1 {
                if byte == characters::GRAVE_ACCENT {
                    concatenation-=1;
                }
            }
            else {
                match byte {
                    characters::APOSTROPHE | characters::DOUBLE_QUOTE => {
                        quote_char=byte;
                    }

                    //If the character is a grave accent OR if the character is a dollar sign followed by a left curly inside a grave accent quoted sentence, increase concatenation by 1.
                    characters::GRAVE_ACCENT | characters::DOLLAR_SIGN  
                    if j < len - 1 &&
                    concatenation & 1 == 1 &&
                    code_analysis_object.current_scope.characters[j+1] == characters::LEFT_CURLY => {
                        concatenation+=1;
                        
                    }
                    //Not a letter nor a number
                    32..=47 | 58..= 63 | 91..=96 | 123..=127 => {
                        next_valid=true;
                    }
                    _ => {
                        for (old_n, new_n) in changed_values {
                            //If the sequence corresponds to the old value and is between other non letter/number characters, replace it with the new name.
                            if next_valid &&
                            byte == old_n[0] &&
                            j < len - 1 &&
                            code_analysis_object.current_scope.characters[j+1] == old_n[1] && 
                            (j<len-2 || match code_analysis_object.current_scope.characters[j+2] {
                                32..=47 | 58..= 63 | 91..=96 | 123..=127 => true, _ => false
                            })
                            {
                                byte=new_n;
                                for char in old_n {
                                     code_analysis_object.character_amounts.remove_small_char(char);
                                }

                                j+=1;

                            }
                        }
                        next_valid=false;
                    }
                }
            }


            new_characters.push(byte);
            code_analysis_object.character_amounts.add_small_char(byte);
            j+=1;
        }


        let mut children_min_value_declaration = code_analysis_object.current_scope.min_value_declarations;

        if code_analysis_object.current_scope.inside_of == code_analysis::InsideOf::ForLoop {
            children_min_value_declaration +=1;
        }

        let all_value_declarations = code_analysis_object.current_scope.all_value_declarations+code_analysis_object.current_scope.values.len() as u16;


        match code_analysis_object.current_scope.end_scope_char {
            code_analysis::EndOfScopeChar::Curly=>code_analysis_object.current_scope.characters.push(characters::RIGHT_CURLY),
            code_analysis::EndOfScopeChar::Parenthesis=>code_analysis_object.current_scope.characters.push(characters::RIGHT_PARENTHESIS),
            _=>{}
        }
        //Re-ordering big if/else statements

        match code_analysis_object.current_scope.inside_of {
            code_analysis::InsideOf::UpperIfStatement(list_scopes,_ )=> {
                let mut curlies = false;
                //Usde to determine if the whole statement has to be used with curlies and "if" and "else" keywords.
                for scope in list_scopes {
                    if scope.end_scope_char == code_analysis::EndOfScopeChar::Curly {
                        curlies=true;
                        break;
                    }
                }
                let multiple = list_scopes.len()>1;
                for scope in list_scopes {
                    //Code to run if the whole scope has to be in curlies
                   if curlies {
                        match scope.inside_of {
                            //Add the "if" keyowrd to the chars 
                            code_analysis::InsideOf::IfStatement=>code_analysis_object.current_scope.characters.append(&mut b"if".to_vec()),
                            //Add the "else if" keyword to the chars
                            code_analysis::InsideOf::IfElseStatement=>code_analysis_object.current_scope.characters.append(&mut b"else if".to_vec()),
                            //Add the "else" keyword to the chars
                            code_analysis::InsideOf::ElseStatement=>code_analysis_object.current_scope.characters.append(&mut b"else".to_vec()),
                            //Supposed to be impossible to have another code_analysis::InsideOf type
                            _=>panic!("Wtf"),
                        }
                        

                        //Add (
                        code_analysis_object.add_one_byte_char(characters::LEFT_PARENTHESIS);

                        //Add the condition 
                        code_analysis_object.current_scope.characters.append(&mut scope.beginning_chars);

                        //Add )
                        code_analysis_object.add_one_byte_char(characters::RIGHT_PARENTHESIS);

                        //Add {
                        code_analysis_object.add_one_byte_char(characters::LEFT_CURLY);

                        //Add the contents of the scope
                        code_analysis_object.code_analysis_object.current_scope.characters.append(&mut scope.characters);

                        //Add }
                        code_analysis_object.add_one_byte_char(characters::RIGHT_CURLY);

                   }
                   //If the scope doesn't have to be in curlies, therefore we can use shortcuts in the syntax.

                   /*If multiple sub scopes are in the if/else statement (Not just one "if"),
                   then use the turnary syntax like this : "condition?code_if_true:code_if_false;"*/
                   else {
                    
                /*In an if/else scope like this, if there is multiple statements in the same scope,
                we seperate them with commas like a normal scope ending with a semi colon, but here we have 
                to add parenthesis. This code verifies if there is more than one sentence.*/

                 let multiple_statements = {
                    if scope.semi_column_indexes.len()>0 {

                        
                    true
                    }
                    else {
                        let mut insides=0;
                        let mut result =true;
                        for char in scope.characters {
                            match char {
                                characters::LEFT_PARENTHESIS | characters::LEFT_SQAURE_BRACKET => {

                                    insides+=1;
                                }
                                characters::RIGHT_PARENTHESIS | characters::RIGHT_SQAURE_BRACKET => {

                                    if insides==0 {
                                        return Err(InvalidSyntaxError);
                                    }
                                    insides-=1;
                                }
                                characters::COMMA => {

                                    if insides==0 {
                                        result=false;
                                    }
                                }
                            }
                            

                        }
                        result
                    }
                 };

                 if multiple {
                    match scope.inside_of {

                        code_analysis::InsideOf::IfStatement=> {
                            //Condition
                            code_analysis_object.current_scope.characters.append(&mut scope.beginning_chars);

                            //"?" character (shortcut for "if" after the condition)
                            code_analysis_object.add_one_byte_char(63);
                        }
                        code_analysis::InsideOf::IfElseStatement => {
                            //":" standing for "else"
                            code_analysis_object.current_scope.characters.append(58);

                            //Then putting another if statement

                            //Condition
                            code_analysis_object.current_scope.characters.append(&mut scope.beginning_chars);

                            //"?" character (shortcut for "if" after the condition)
                            code_analysis_object.add_one_byte_char(63);

                            
                        } 
                        code_analysis::InsideOf::ElseStatement => {
                            code_analysis_object.add_one_byte_char(58);
                        }

                    }
                    

                    /*If there is a single "if" statement, we simply use this syntax: "condition&&code;"

                    Using the and logic, if the first condition is false, it wont execute whatevers comes next.
                    Because of this, simply using the && with the code to execute after is the shortest substitue
                    for a simple if statement. */
                   } else {
                    
                    for index in scope.semi_column_indexes {
                        scope.characters[index]=44;
                    }
                    //Add condition
                    code_analysis_object.current_scope.characters.append(&mut scope.beginning_chars);

                    //Add "&&"
                    code_analysis_object.add_one_byte_char(38);
                    code_analysis_object.add_one_byte_char(38);

                    //Add code 


                   }
                   if multiple_statements{code_analysis_object.add_one_byte_char(40);}
                    code_analysis_object.current_scope.characters.append(&mut scope.characters);
                    if multiple_statements{code_analysis_object.add_one_byte_char(characters::RIGHT_PARENTHESIS);}
                   
                   
                }

            }
        }
                 
                   

        }


        let parent = scope.borrow();
        let mut parent_clone = parent.clone();
        drop(parent);


        //If the scope is a function, we need to add the potential values declared to the potential values declared.
        //Else, we need to add the potential values called to the potential values called,


        match code_analysis_object.current_scope.inside_of {
            code_analysis::InsideOf::Function(x) => parent_clone.potential_values_declared.push((x, code_analysis_object.current_scope.potential_values_called)),
            _=> parent_clone.potential_values_called.append(&mut code_analysis_object.current_scope.potential_values_called),
        };
        parent_clone.potential_values_declared.append(&mut code_analysis_object.current_scope.potential_values_declared);

         //If the parent scope is a grouped if/else statement 
         match &mut parent_clone.inside_of {
            code_analysis::InsideOf::UpperIfStatement(scopes,_ ) => {
                scopes.push(code_analysis_object.current_scope.clone());
            }
            _=> {
                parent_clone.characters.append(&mut code_analysis_object.current_scope.beginning_chars);
                parent_clone.characters.append(&mut code_analysis_object.current_scope.characters);
            }
         }
    
        
        *code_analysis_object.current_scope = parent_clone;

        code_analysis_object.current_scope.min_value_declarations += children_min_value_declaration;
        code_analysis_object.current_scope.all_value_declarations = all_value_declarations;

        code_analysis_object.current_scope.is_a_parent = true;


        code_analysis_object.current_scope.children_used_values = all_children_used_values;
            
     }
     Ok(())

}


fn drain_values(buf_bytes: &mut Vec<u8>, i: &mut usize, last_index: &mut usize) {
    if *last_index+1 != *i {
        buf_bytes.drain(*last_index+1..*i);
        *i=*last_index;
    } else {
        *last_index=*i;
    }
}

pub async fn compress_js(file_input: &str, file_output: &str) -> io::Result<()> {
    //1. Remove comments
    //2. Minimify functions/variable/class/element names so they can all be one byte character long if possible.

    let mut amount_occurences: [u16;128] = [0;128];
    let mut shortcut_eight_byte_chars: Vec<u8> = Vec::new();
    let mut free_eight_byte_characters: Vec<u8> = Vec::new();
    let mut content_bound_w_eight_bytes: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut content_occurences: Vec<u8> = Vec::new();

    let mut least_recurring_char: usize = 0;

    let mut larger_content: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut larger_shortcut: Vec<[u8;2]> = Vec::new();
    let mut taken_2_byte_chars: HashMap<[u8;2], u16> =  HashMap::new();
    let mut latest_available_2_byte: [u8;2] = [192, 128];

    let mut taken_3_byte_chars: HashMap<[u8;3], u16> =  HashMap::new();
    let mut taken_4_byte_chars: HashMap<[u8;4], u16> = HashMap::new();
    
    let mut calculate_amount = || {
        let mut all_texts: Vec<String> = [content_bound_w_eight_bytes.borrow().clone(), larger_content.borrow().clone()].concat();
        all_texts.push(buffer.borrow().clone());
        for text in all_texts {
            let mut i=0;
                    let bytes = text.to_string().clone().into_bytes();
                    let len = bytes.len();

                    while i < len {
                        let byte = bytes[i];
                        if byte < 128{
                            amount_occurences[byte as usize]+=1;
                            i+=1;
                        }
                        else if byte >= 192 && byte < 224 {
                            let character = [byte, bytes[i+1]];
                            taken_2_byte_chars.insert(character, match taken_2_byte_chars.get(&character){Some(x)=>x+1,None=>1});
                            i+=2;
                        } else if byte >= 224 && byte < 240 {
                            let character = [byte, bytes[i+1], bytes[i+2]];
                            taken_3_byte_chars.insert(character, match taken_3_byte_chars.get(&character){Some(x)=> *x+1,None=>1});
                            i+=3
                        } else {
                            let character = [byte, bytes[i+1], bytes[i+2], bytes[i+3]];
                            taken_4_byte_chars.insert(character, match taken_4_byte_chars.get(&character){Some(x)=> *x+1,None=>1});
                            i+=4;
                        }
            }
        }
    };

    calculate_amount();

    for i in 0..128{if amount_occurences[i]==0{free_eight_byte_characters.push(i as u8)}}
    let mut replace_everywhere = |from: &str, to: &str| {
        let mut buffer_ref = buffer.borrow_mut();
        *buffer_ref = buffer_ref.replace(from, to);

        let mut content_vec = content_bound_w_eight_bytes.borrow_mut();
        for s in content_vec.iter_mut() {
            *s = s.replace(from, to);
        }

        let mut larger_vec = larger_content.borrow_mut();
        for s in larger_vec.iter_mut() {
            *s = s.replace(from, to);
        }
    };
    for mat in MATCHES {
        if buffer.borrow().contains(mat) {
            match free_eight_byte_characters.pop() {
                Some(x) => {
                    let char = x;
                    replace_everywhere(mat,str::from_utf8(&[char]).unwrap());
                    shortcut_eight_byte_chars.push(char);
                    Rc::clone(&content_bound_w_eight_bytes).borrow_mut().push(mat.to_string());
                } None => {
                    least_recurring_char = 128;
                    let amount_mat = buffer.borrow().match_indices(mat).count() as u16;
                    let mut least_amount= amount_mat;
                    for i in 0..128 {
                        if least_amount>amount_occurences[i] {
                            least_recurring_char=i;
                            least_amount=amount_occurences[i];
                        }
                    }
                    if least_recurring_char != 128 {
                        let mut find_two_byte_char = |el_to_rem: &str| {
                            loop {
                                if latest_available_2_byte[1]==255 {
                                    latest_available_2_byte = [latest_available_2_byte[0]+1,128]
                                } else {
                                    latest_available_2_byte[1]+=1;
                                }
                                if !taken_2_byte_chars.contains_key(&latest_available_2_byte) {
                                    larger_shortcut.push(latest_available_2_byte);
                                    Rc::clone(&larger_content).borrow_mut().push(el_to_rem.to_string());
                                    replace_everywhere(str::from_utf8(&[least_recurring_char as u8]).unwrap(), str::from_utf8(&latest_available_2_byte[..]).unwrap());
                                    break;
                                }
                            }
                        };
                        if shortcut_eight_byte_chars.contains(&(least_recurring_char as u8)) {
                        let pos = shortcut_eight_byte_chars.iter().position(|n| n == &(least_recurring_char as u8)).unwrap();
                        let el = content_bound_w_eight_bytes.borrow()[pos].clone();

                        if el.len()as u16*least_amount>2*(least_amount+1)+4 {
                            find_two_byte_char(&el);
                        }
                        
                        Rc::clone(&content_bound_w_eight_bytes).borrow_mut()[pos] = mat.to_string();
                        replace_everywhere( mat, str::from_utf8(&[shortcut_eight_byte_chars[pos]]).unwrap());
                        
                    } else if (least_amount+1)*2+4<amount_mat {
                        let mut els = String::new();
                        els = str::from_utf8(&[least_recurring_char as u8]).unwrap().to_string();
                        let el = els.as_str();
                
                        find_two_byte_char(el);
                    }

                    }
                }
            }
        }
    }
        
    //Remove spaces, newlines and comments
    let re = Regex::new(r"<.*>([^<]+)</.*>").unwrap();
    let buffer_ref = buffer.borrow();
    let mut buffer_mut = buffer.borrow_mut();

    for ((full, [text])) in re.captures(&*buffer_ref).map(|c| c.extract()) {
       *buffer_mut = buffer_mut.replace(full, full.replace(text, text.replace(" ", "ķ").replace("\n", "Ĺ").as_str()).as_str());
    }

    let mut quote_char: u8 = 0;
    let mut buf_bytes: Vec<u8> = Vec::new();
    let mut i = 0;
    let mut last_index = 0;
    let mut index_difference = 0;
    
    let mut inside_for = false;

    let mut all_values_count: HashMap<code_analysis::NewValueName, u16> = HashMap::new();

    let file = File::open(file_input).await?;


    let (tx, mut rx) = mpsc::channel::<[u8;1024]>(4);


    let not_inside_quote= Rc::new(RefCell::new(false));
    let not_inside_quote_clone = Rc::clone(&not_inside_quote);

    tokio::spawn(async move {
        let mut line_to_send: Vec<u8> = Vec::new();
        let mut buffer  :[u8;1024]= [0;1024];
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 || tx.send(buffer.clone()).await.is_err() {
                break;
            }

        }
    });

    let mut resuming_list: Vec<u8> = Vec::new();  

    let mut going_through = code_analysis::GoingThrough::HTML;
    let was_originally_html = {
        if file_input.ends_with(".html") {
            true
        } 
        else if file_input.ends_with(".css") {
            going_through = code_analysis::GoingThrough::CSS;
            false
        }
        else if file_input.ends_with(".js") {
            going_through =code_analysis::GoingThrough::JS;
            false
        } else {
            false
        }
        
        
    };
    const length: usize = 1024;

    let mut code_analysis_object = {
        code_analysis::CodeAnalysisObject {
            previous_buffer: [0;1024],
            current_buffer: [0;1024],
            buffer_ref: &[0;1024],
            going_to_current: true,
            last_index: 0,
            i_buffer: 0,
            last_char: 0,
            current_scope: code_analysis::JsScope::new(None,   code_analysis::InsideOf::NormalCode),
            character_amounts: compression::CharacterAmounts { 
                one_byte_chars: [0;127],
                two_byte_chars: std::array::from_fn(|_| Vec::with_capacity(4)),
                three_byte_chars: std::array::from_fn(|_| Vec::with_capacity(4)),
                four_byte_chars: std::array::from_fn(|_| Vec::new()) 
            },
            current_big_char: [0;4],
            current_char_bytes_remaining: 0, 
            previous_buffer_state: code_analysis::PreviousBufferState::Code,

        }
    };

    while let Some(buffer_bytes) = rx.recv().await {

       
        code_analysis_object.previous_buffer = code_analysis_object.current_buffer;

        code_analysis_object.current_buffer = buffer_bytes;
        drop(code_analysis_object.buffer_ref);
        (code_analysis_object.buffer_ref, code_analysis_object.i_buffer) = if code_analysis_object.going_to_current {
            (&code_analysis_object.current_buffer, 0)
        } else {
            (&code_analysis_object.previous_buffer, code_analysis_object.last_index)
        };
        
        code_analysis_object.i_buffer = if code_analysis_object.going_to_current {0} else {};

        let mut continuing = true;

        match code_analysis_object.previous_buffer_state {

            code_analysis::PreviousBufferState::WaitingIfStatementFirstI => code_analysis_object.verify_if_else_statement(false),

            code_analysis::PreviousBufferState::WaitingIfStatementFà => code_analysis_object.verify_if_else_statement(true),

        }
        
        while code_analysis_object.check_index(){
            

            let byte = code_analysis_object.get_char();
            
            if quote_char!=0 {

                //bad bad bad to change
               find_quote_chars(&code_analysis_object.buffer_ref, &mut code_analysis_object.current_scope, &mut code_analysis_object.i_buffer, &mut quote_char, &mut code_analysis_object.previous_buffer_state, &mut going_through, last_index);
            }

            else {
                match code_analysis_object.last_char {

                    //If the last character was a space and the character before it is a letter AND this character is also a letter, add the space.
                    characters::SPACE => {
                        match byte {
                            
                            32 | 34..=47 | 58..=63 | 91 | 93 | characters::LEFT_CURLY..=characters::RIGHT_CURLY => {}
                            _ => {
                                code_analysis_object.add_one_byte_char(characters::SPACE);
                            }
                        }

                    }
                    characters::EQUAL => {
                        match byte {
                            characters::MORE_THAN => {
                                //=>
                            }
                            characters::LEFT_CURLY => {
                                //class declaration
                            }

                        }
                        
                    }
                }
                code_analysis_object.last_char=0;
                

                code_analysis_object.last_index = code_analysis_object.i_buffer;
                let mut iterated=false;

                loop {

                    match code_analysis_object.get_char() {
                        0..=47 | 58..=64 | 91 | 93 | 94 | characters::LEFT_CURLY..=128 => {
                            //This ensures that if its a single character, it gets included.
                            break;
                        }
                        _=> {
                            code_analysis_object.i_buffer+=1;
                        }
                    }
                    iterated=true;
                    continuing = code_analysis_object.check_index();
                    if !continuing {
                        break;
                    }
                }
                if !continuing && iterated {
                    break;
                }

                //If single character 
                if code_analysis_object.i_buffer < code_analysis_object.last_index+2 {
                    let value: u8 = code_analysis_object.buffer_ref[code_analysis_object.last_index];

                    match value {
                        //If the value is a number, simply add it. No more processing needed.
                        48..57=> {code_analysis_object.add_one_byte_char(value)}

                        characters::NEWLINE => {}

                        characters::SPACE => {
                            match code_analysis_object.previous_buffer_state {
                                 code_analysis::PreviousBufferState::ExpectingScopeChar | code_analysis::PreviousBufferState::WaitingParenthesis => {},
                                _ => {
                                    match if code_analysis_object.i_buffer !=0 {code_analysis_object.buffer_ref[code_analysis_object.i_buffer-1]} 
                                          else if code_analysis_object.going_to_current {code_analysis_object.previous_buffer[1023]}
                                          else {32} {
                                          32 | 34..=47 | 58..=63 | 91 | 93 | characters::LEFT_CURLY..=characters::RIGHT_CURLY => {}
                                          _ => {
                                            code_analysis_object.last_char = characters::SPACE;
                                          }

                                    }
                                }
                            }
                        }
                        
                        characters::APOSTROPHE | characters::DOUBLE_QUOTE | characters::GRAVE_ACCENT => {
                            code_analysis_object.add_one_byte_char(byte);
                            quote_char = byte;
                        }

                        characters::SLASH => {
                            //If its a double slash, then it's a single line JS Comment and we want to not add the characters inside of it.
                            if code_analysis_object.last_char == characters::SLASH {
                                //Keep iterating without adding the character to the vector until we hit newline
                                code_analysis_object.go_through_js_inline_comment();
                            }
                            else {
                                code_analysis_object.last_char=characters::SLASH;
                            }
                            
                        }
                        characters::ASTERIS => {
                            //If the asteris is preceeded by a slash, its a multi line JS Comment.
                            if code_analysis_object.last_char == characters::SLASH {
                                code_analysis_object.go_through_js_comment();
                            }

                        }
                        
                    }
                    
                //If multiple characters 

                } else {
                    let value: Vec<u8> =code_analysis_object.buffer_ref[code_analysis_object.last_index..code_analysis_object.i_buffer+1];

                    match value {
                        b"if" => {
                            for inside in [code_analysis::InsideOf::UpperIfStatement(Vec::new(), false), code_analysis::InsideOf::IfStatement] {
                                code_analysis_object.current_scope = code_analysis::JsScope::new(Some(code_analysis_object.current_scope), inside);
                            }
                            code_analysis_object.previous_buffer_state = code_analysis::PreviousBufferState::WaitingParenthesis;

                        }
                        b"else" => code_analysis_object.verify_if_else_statement(false),

                        b"function" => {
                        code_analysis_object.find_function_name();
                        while code_analysis_object.buffer_ref[code_analysis_object.i_buffer] != 40 {
                            code_analysis_object.i_buffer+=1;
                        }
                        code_analysis_object.add_one_byte_char(40);

                        code_analysis_object.new_values_init();
                            
                        }
                        b""
                        
                    }
                }
               
                match byte {
                    characters::NEWLINE=> {}

                    characters::SPACE=> {
                        match code_analysis_object.previous_buffer_state {
                             code_analysis::PreviousBufferState::ExpectingScopeChar | code_analysis::PreviousBufferState::WaitingParenthesis => {},
                            _ => {
                                if code_analysis_object.i_buffer != 0 && code_analysis_object.i_buffer != length-1 {
                                    match code_analysis_object.buffer_ref[code_analysis_object.i_buffer-1] {
                                        32 | 34..=47 | 58..=63 | 91 | 93 | characters::LEFT_CURLY..=characters::RIGHT_CURLY => {}
                                        _ => {
                                            match code_analysis_object.buffer_ref[code_analysis_object.i_buffer+1] {
                                            32 | 34..=47 | 58..=63 | 91 | 93 | characters::LEFT_CURLY..=characters::RIGHT_CURLY => {} 
                                            _ =>{
                                                    code_analysis_object.add_one_byte_char(32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                    }

                    characters::APOSTROPHE | characters::DOUBLE_QUOTE | characters::GRAVE_ACCENT => {
                        code_analysis_object.add_one_byte_char(byte);
                        quote_char = byte;
                    }
                    
                    //Javascript/CSS Comment
                    
                    //HTML comment (if not inside concatened string)
                    60 => {
                        if find_if_corresponds(&code_analysis_object.buffer_ref, &mut code_analysis_object.i_buffer, vec![33, 45, 45]) {
                            loop {
                                code_analysis_object.i_buffer+=1;
                                if code_analysis_object.i_buffer==length {
                                    code_analysis_object.previous_buffer_state = code_analysis::PreviousBufferState::HTMLComment;
                                    break;
                                }
                                if code_analysis_object.buffer_ref[code_analysis_object.i_buffer] == 45 && find_if_corresponds(&code_analysis_object.buffer_ref, &mut code_analysis_object.i_buffer, vec![45, 62]) {
                                    break;

                                }
                            }
                        }
                    }
                    characters::RIGHT_CURLY=> {

                        while code_analysis_object.current_scope.end_scope_char == code_analysis::EndOfScopeChar::SemiColon {
                            scope_end(&mut code_analysis_object.current_scope, &mut all_values_count).await;
                        }

                        if code_analysis_object.current_scope.end_scope_char == code_analysis::EndOfScopeChar::Curly {
                            
                            //ANALYSIS 9.1 (See doc for more info)
                            /*Analyze if we can replace curly brackets in a scope */

                            let changing_to_semi_column = code_analysis_object.current_scope.values.len() as u16 == code_analysis_object.current_scope.min_value_declarations && (!code_analysis_object.current_scope.is_a_parent || code_analysis_object.current_scope.semi_column_indexes.len() == {if code_analysis_object.last_char_semi_column {0} else {1}});
                            if changing_to_semi_column {
                                code_analysis_object.current_scope.end_scope_char = code_analysis::EndOfScopeChar::SemiColon;

                                if *code_analysis_object.current_scope.characters.last().unwrap() == 59 { code_analysis_object.current_scope.semi_column_indexes.pop(); } else { code_analysis_object.add_one_byte_char(59); }

                                //As long as the scope isn't inside an if/else statement, change all the semicolons by commas
                                match code_analysis_object.current_scope.inside_of {
                                    code_analysis::InsideOf::ElseStatement | code_analysis::InsideOf::IfElseStatement | code_analysis::InsideOf::IfStatement =>{},

                                    _ => {
                                        for index in code_analysis_object.current_scope.semi_column_indexes {

                                            //TEST (Remove at production)
                                            if code_analysis_object.current_scope.characters[index] != 59 {
                                                panic!("INDEX OF SEMI COLUMN DOES NOT POINT TO SEMI COLUMN");
                                            }
                                            code_analysis_object.current_scope.characters[index] = 44;
                                        }
                                    }
                                }
                                
                                
                            } else {
                                code_analysis_object.add_one_byte_char(characters::RIGHT_CURLY)
                            }

                            scope_end(&mut code_analysis_object.current_scope, &mut all_values_count, &mut code_analysis_object.previous_buffer_state).await;
                            
                            
                        }

                    }
                    
                    

                    //=>
                   
                    //if
                    105 if find_if_corresponds(&code_analysis_object.buffer_ref, &mut code_analysis_object.i_buffer, vec![102], &mut code_analysis_object.previous_buffer_state) => {
                        
                        
                        
                    }
                    
                    //function
                    102 if find_if_corresponds(&code_analysis_object.buffer_ref, &mut code_analysis_object.i_buffer, vec![117,110,99,116,105,111,110,32], &mut code_analysis_object.previous_buffer_state) {
                        
                        
                    }



                    
                    //;
                    59 => {
                        while code_analysis_object.current_scope.end_scope_char == code_analysis::EndOfScopeChar::SemiColon {
                            scope_end(&mut code_analysis_object.current_scope, &mut all_values_count, &mut code_analysis_object.previous_buffer_state);
                        }
                        if code_analysis_object.current_scope.end_scope_char == code_analysis::EndOfScopeChar::Curly {
                            code_analysis_object.current_scope.semi_column_indexes.push(code_analysis_object.current_scope.characters.len());
                            
                        } 
                        code_analysis_object.add_one_byte_char(59);
                        
                    }
                    //)
                    characters::RIGHT_PARENTHESIS => {
                        if code_analysis_object.current_scope.end_scope_char == code_analysis::EndOfScopeChar::Parenthesis {
                            scope_end(&mut code_analysis_object.current_scope, &mut all_values_count, &mut code_analysis_object.previous_buffer_state);
                            code_analysis_object.add_one_byte_char(characters::RIGHT_PARENTHESIS);

                       
                        } else if let code_analysis::PreviousBufferState::WaitingEndScopeChar(x) = &code_analysis_object.previous_buffer_state {
                                
                                if *x == 0 {
                                    drop(x);
                                    code_analysis_object.previous_buffer_state = code_analysis::PreviousBufferState::ExpectingScopeChar;
                                }
                                *x-=1;
                            
                        } else {
                            code_analysis_object.add_one_byte_char(characters::RIGHT_PARENTHESIS);
                        }
                        
                    }
                    //(
                    40 => {
                        match &code_analysis_object.previous_buffer_state {
                            code_analysis::PreviousBufferState::WaitingParenthesis => code_analysis_object.previous_buffer_state = code_analysis::PreviousBufferState::WaitingEndScopeChar(0),
                            code_analysis::PreviousBufferState::WaitingEndScopeChar(x) => {
                                *x+=1;
                                code_analysis_object.add_one_byte_char(40);
                            },
                            code_analysis::PreviousBufferState::ExpectingScopeChar => {
                                assign_end_scope_char(&mut code_analysis_object.current_scope, &mut code_analysis_object.previous_buffer_state, code_analysis::EndOfScopeChar::Parenthesis);
                            }
                            _=>{},
                        }
                    }
                }
            }
            
            match byte {
                32 | 10 => {},
                _=> {
                    match code_analysis_object.current_scope.inside_of {
                        code_analysis::InsideOf::UpperIfStatement(_,_)=>{
                            code_analysis_object.previous_buffer_state =  code_analysis::PreviousBufferState::Code;
                            scope_end(&mut code_analysis_object.current_scope, &mut all_values_count, &mut code_analysis_object.previous_buffer_state);
                        } ,
                        _=> {
                            if code_analysis_object.previous_buffer_state == code_analysis::PreviousBufferState::ExpectingScopeChar {
                                assign_end_scope_char(&mut code_analysis_object.current_scope, &mut code_analysis_object.previous_buffer_state, code_analysis::EndOfScopeChar::SemiColon);
                            }
                        }
                    }
                }
            }
            code_analysis_object.i_buffer+=1;
         
        }
        
    }


    

    while i < buf_bytes.len() {
        let byte = buf_bytes[i];
        if quote_char!=0 {
            
       
        } else {
            
            match byte { 
                //function => 0
                //let => 1
                //const => 2
                //var => 3

                10 => {}
                32 => {
                }
                34 | 39 | 96 => {
                    quote_char = byte;
                } 47 => {
                    match buf_bytes[i+1] {
                        47 => {
                            loop {
                                i+=1;
                                if buf_bytes[i] == 10 {
                                    break;
                                }
                            }
                        } 
                        42 => {
                            loop {
                                i+=1;
                                if buf_bytes[last_index] == 42  {
                                    if buf_bytes[last_index+1] == 47 {
                                        i+=1;
                                        break;
                                    }
                                }
                            }
                        } _ => {drain_values(&mut buf_bytes, &mut i, &mut last_index);}
                    }
                }
               
                characters::RIGHT_CURLY => {
                    
                    if let Some(scope) = code_analysis_object.current_scope.parent_scope {
                        let all_children_used_values = code_analysis_object.current_scope.children_used_values.clone();
                        let mut unused_values: HashMap<u8, Option<usize>> = HashMap::new();

                        for val in all_values_count.keys().cloned() {
                            if let code_analysis::NewValueName::OneChar(one_char) = val {
                                
                                let mut unused=true;
                                for value in &code_analysis_object.current_scope.values {
                                    if value.value_new_name == val {
                                        unused=false;
                                        break;
                                    }
                
                                }
                                
                                if unused {
                                    let mut present_after: bool = false;
                                    for child_value in code_analysis_object.current_scope.children_used_values.clone() {
                                        if child_value.value_new_name == val {
                                            unused_values.insert(one_char, Some(child_value.last_usage_index));
                                            present_after=true;
                                            break;
                                        }
                                    }
                                    if !present_after {
                                        unused_values.insert(one_char, None);
                                    }
                                    
                                }
                            }
                            
                        }
                        let mut changed_values: HashMap<[u8;2], u8> = HashMap::new();
                        
                        for value in &mut code_analysis_object.current_scope.values {
                            if let code_analysis::NewValueName::TwoChar(two_char) =value.value_new_name {
                            let mut changed_value : Option<u8> = None;
                            
                            for (new_val, index) in &unused_values {
                                /*If the last time the value was called is before the assignment
                                 of the value we want to replace, then we can go ahead and replace it
                                 without affecting the functionnality of the code. */
                                if match index {
                                   Some(i)=>{*i<value.declaration_index }
                                   None=>{true}
                                }{
                                    changed_values.insert(two_char, *new_val);
                                    changed_value = Some(*new_val);
                                    value.value_new_name = code_analysis::NewValueName::OneChar(*new_val);
                                    break;
                                }
                                    
                                }
                                if let Some(v) = changed_value {
                                    unused_values.remove(&v);
                                }
                            }
                        }

                        let mut j = code_analysis_object.current_scope.starting_index;
                        let mut q_char: u8 = 0;
                        loop {
                            {
                                let len = code_analysis_object.current_scope.characters.len();
                                if j >= len {
                                    break;
                                }
                            }
                            // Now only mutable borrow
                            find_value_to_replace(&mut q_char, &mut j, &changed_values, code_analysis_object.current_scope);
                            tokio::task::yield_now();
                        }
                        let parent = scope.borrow();
                        
                        code_analysis_object.current_scope = parent.clone();
                        code_analysis_object.current_scope.children_used_values = all_children_used_values;
                            
                     }
                    }
                
                characters::LEFT_CURLY => {
                    if !inside_for {
                        code_analysis_object.current_scope = code_analysis::JsScope::new(Some(code_analysis_object.current_scope), i, false);
                    } else {
                        inside_for = false;
                    }
                }
                //=>
                61 => {
                    drain_values(&mut buf_bytes, &mut i, &mut last_index);

                    if buf_bytes[i+1] == 62 {
                        let mut j =i;
                        code_analysis_object.current_scope = code_analysis::JsScope::new(Some(code_analysis_object.current_scope), i,  false);
                        
                        while buf_bytes[j] != 40 && buf_bytes[j] != 61 {
                            j-=1;
                        }
                        let mut param_name: Vec<u8> = Vec::new();
                        loop {
                            match buf_bytes[j] {
                                28 => {
                                    add_value(&mut code_analysis_object.current_scope, &mut all_values_count,  std::mem::take(&mut param_name), i);
                                    
                                } 
                                61 | characters::RIGHT_PARENTHESIS => {
                                    add_value(&mut code_analysis_object.current_scope, &mut all_values_count, param_name, i);
                                    break;
                                } 
                                _ => {
                                    param_name.push(buf_bytes[j]);
                                }
                            }
                        }
                        i+=1;
                        last_index+=1;
                        let mut to_empty=false;
                        while buf_bytes[i+1] == 32 || buf_bytes[i+1] == 10 {
                            i+=1;
                            to_empty=true;
                        }
                        if to_empty {
                            buf_bytes.drain(last_index+1..i);
                            i=last_index;
                        }
                        if buf_bytes[i+1] == characters::LEFT_CURLY {
                            i+=1;
                            last_index+=1;
                        }
                    }
                }
                 _ => {

                    drain_values(&mut buf_bytes, &mut i, &mut last_index);
                    //const 
                    if byte == 99 && buf_bytes[i+1] == 110 && buf_bytes[i+2] == 111 && buf_bytes[i+3] == 115 && buf_bytes[i+4] == 116 && buf_bytes[i+5] == 32 {
                        last_index+=5;
                        i+=5;
                        let mut end_index=i;
                        let mut value_name: Vec<u8> = Vec::new();

                        const_fn(&mut FnParams {
                                i: &mut i,
                                end_index: &mut end_index,
                                buf_bytes: &mut buf_bytes,
                                value_name: &mut value_name,
                                all_values_count: &mut all_values_count,
                                last_index: &mut last_index,
                                code_analysis_object.current_scope: &mut code_analysis_object.current_scope,
                        });
                    }
                    //function
                    else if byte == 102 && buf_bytes[i+1] == 117 && buf_bytes[i+2] == 111 && buf_bytes[i+3] == 99 && buf_bytes[i+4] == 116 && buf_bytes[i+5] == 105 && buf_bytes[i+6] == 111 && buf_bytes[i+7] == 110 && buf_bytes[i+8] == 32 {
                        i+=8;
                        last_index+=8;
                        let mut end_index=i;
                        let mut function_name: Vec<u8> = Vec::new();

                        loop {
                            end_index+=1;
                            match buf_bytes[end_index] {
                                40 => {
                                    add_value(&mut code_analysis_object.current_scope, &mut all_values_count, function_name, i);
                                    break;
                                    
                                } _ => {
                                    function_name.push(buf_bytes[i]);
                                }
                            }
                        }

                        

                    }
                    //let
                    else if byte == 108 && buf_bytes[i+1] == 101 && buf_bytes[i+2] == 116 && buf_bytes[i+3] == 32 {
                        i+=3;
                        last_index+=3;
                        let mut end_index = i;
                        let mut value_name: Vec<u8> = Vec::new();



                        const_fn(&mut FnParams {i: &mut i,
                                                end_index: &mut end_index,
                                                buf_bytes: &mut buf_bytes,
                                                value_name: &mut value_name,
                                                all_values_count: &mut all_values_count,
                                                last_index: &mut last_index,
                                                code_analysis_object.current_scope: &mut code_analysis_object.current_scope});
                        
                    }
                    //var
                    else if byte == 118 && buf_bytes[i+1] == 97 && buf_bytes[i+2] == 114 && buf_bytes[i+3] == 32 {

                    }
                    //for
                    else if byte == 102 && buf_bytes[i+1] == 111 && buf_bytes[i+2] == 114 {
                        i+=3;
                        last_index=i-1;
                        let mut did = false;
                        while buf_bytes[i] == 32 {
                            i+=1;
                            did=true;
                        }
                        if did {
                            buf_bytes.drain(last_index+1..i);
                            i=last_index;
                        }
                        if buf_bytes[i+1] == 40 
                        {
                            code_analysis_object.current_scope = code_analysis::JsScope::new(Some(code_analysis_object.current_scope),  false, code_analysis::InsideOf::ForLoop);
                            inside_for= true;
                            i+=1;
                        }
                    } else {
                       find_value(&mut buf_bytes, &mut code_analysis_object.current_scope, &mut i, &mut last_index, &mut all_values_count);
                    }

                 }

            }
        }
        i+=1;
    }

    *buffer_mut = buffer_mut.replace("ķ", " ").replace("Ĺ", "\n");
    Ok(())
}

fn ajouter(a: i32, b: i32 )-> i32 {
    return a+b;
}
fn multiplier(a: i32,b: i32)-> i32 {
    return a*b;
}
fn soustraire(a: i32, b:i32)-> i32 {
    return a-b;
}



pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    



    use super::*;

    #[test]
    fn test() {
        //Array statique de fonctions 
        let liste_de_fonctions: [fn (i32,i32) -> i32; 3] = [ajouter, multiplier, soustraire];
        let mut i =0;
        while i < 3 {
            let fonction_a_executer: fn(i32, i32) -> i32 = liste_de_fonctions[i];
            let resultat = fonction_a_executer(4, 5);
            println!("Le résultat est {}", resultat);
            i+=1;
        }
        
    }
}


//TODO
//Add something that gets all the used values from a function when its called.

/*Current issue, lets consider this code:

    let value1 = 5;

    function myFunc() {
        console.log(value1);
    }

    let value2 = 2;

    myFunc();


With the current code's logic, value2 could have the same one char value name as value1, but because we then call
a function using value1, its gonna be replaced by value2 which is not what we want.

We need a data structure to track all of the used values of functions that could potentially be called
in order to update the "last_usage_index" property of each value used inside the function each time the function gets called.

*/