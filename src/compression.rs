/*This file contains all of the structures and enums necessary in order to compress the file 
after code analysis via pattern identification and use of unused UTF-8 characters. */
pub struct MediumCharacterAmount {
    pub character_subindex: u8,
    pub amount: u16,
}

pub struct FourCharacterAmount {
    pub character_subindexes: [u8;2],
    pub amount: u16,
}
pub struct CharacterAmounts {
    pub one_byte_chars: [u32;128],
    pub two_byte_chars: [Vec<MediumCharacterAmount>;256],
    pub three_byte_chars: [Vec<MediumCharacterAmount>;256],
    pub four_byte_chars: [Vec<FourCharacterAmount>;256],
}

impl CharacterAmounts {
    ///Increments the amount of the corresponding one byte char by 1.
    /// 
    ///Time complexity: O(1)
    pub fn add_small_char(&mut self, byte: u8) {
        self.one_byte_chars[byte as usize]+=1;
    }
    pub fn remove_small_char(&mut self, byte: u8) {
        
    }

    ///Increments the amount of corresponding two byte or more character.
    /// 
    ///Time complexity: O(n) where n is the length of the map bucket.
    pub fn add_big_char(&mut self, bytes: [u8;4]) {
        //If the thrid byte is null (0), we know the character is 2 bytes long.
        if bytes[2] == 0 {
            //Two byte char pub structure: 0b'110abcde 1fghijkl' 
            //List index: 0b'efghijkl'
            let list_index = (bytes[1] & 0b01111111 | (bytes[0] << 7)) as usize;
            //Subindex: 0b'110abcde'
            let subindex = bytes[0];
                                                     
            for value in &mut self.two_byte_chars[list_index] {
                if value.character_subindex == subindex {
                    *value.amount+=1;
                    return;
                } 

            }
            self.two_byte_chars[list_index].push(MediumCharacterAmount { character_subindex: subindex, amount: 1 })
        //Else if the fourth byte is null, the length is 3.
        } else if bytes[3] == 0 {
            //Three byte char pub structure: 0b'1110abcd 10efghij 10klmnop'
            //List index: 0b'ijklmnop'
            let list_index = (bytes(2) & 0b00111111 | (bytes[1] << 6)) as usize;
            //Subindex: 0b'abcdefgh'
            let subindex = (bytes[0] << 4) | ((bytes[1] >> 2) & 0b00111111);
            for value in &mut self.three_byte_chars[list_index] {
                if value.character_subindex == subindex {
                    *value.amount+=1;
                    return;
                }
            }
            self.three_byte_chars[list_index].push(MediumCharacterAmount { character_subindex: subindex, amount: 1 })

        //Else, length is four.    
        } else {
            //Four byte char pub structure: 0b'11110abc 10defghi 10jklmno 10pqrstu'
            //List index: 0b'bcpqrstu'
            let list_index = (bytes[3] & 0b00111111 | (bytes[0] << 6)) as usize;

            //Subindexes: 0b'1adefghi 10jklmno'

            let sub_indexes: [u8; 2] = [((bytes[0] << 4) & 0b01000000) | bytes[1], bytes[2]];

            for value in &mut self.four_byte_chars[list_index] {
                if value.character_subindexes = sub_indexes {
                    *value.amount+=1;
                    return;
                }
            }

            self.four_byte_chars[list_index].push(FourCharacterAmount { character_subindexes: sub_indexes, amount: 1 })

        }

    }

    /// Converts the two bytes character from index/subindex form and returns its original two byts utf 8 array.
    /// Sequence : 0b'efghijkl', 0b'110abcde' -> 0b'110abcde 1fghijkl' 
    /// 
    /// Time complexity: O(1)
    pub fn extract_two_bytes_char(index: usize, subindex: u8)-> [u8;2] {

        return [
            subindex,
            index | 0b10000000
        ];
    }

    /// Converts the three bytes character from index/subindex form and returns its original three bytes utf 8 array.
    /// 
    /// Time complexity: O(1)
    pub fn extract_three_bytes_char(index: usize, subindex: u8) -> [u8;3] {
        return [(subindex >> 4) | 0b11100000,
         ((subindex << 2 & 0b00111111) | 0b10000000) | (index >> 6),
         (index & 0b00111111) | 0b10000000
         ];

    }

    /// Converts the four bytes character from index/subindex form and returns its original four bytes utf 8 array.
    /// 
    /// Time complexity: O(1)
    pub fn extract_four_byte_char(index: usize, subindexes: [u8;2]) -> [u8;4] {
        return [
            ((subindexes[0] >> 4) & 0b00000100) | ((index >> 6) & 0b00000011) | 0b11110000,
            subindexes[0] & 0b10111111,
            subindexes[1],
            (index & 0b00111111) | 0b10000000
        ]

    }


}
