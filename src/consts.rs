/*

	PreSongSelectAddresses
	Base        int64 `sig:"F8 01 74 04 83 65"`
	MenuMods    int64 `sig:"C8 FF ?? ?? ?? ?? ?? 81 0D ?? ?? ?? ?? 00 08 00 00"`
	PlayTime    int64 `sig:"5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04"`
	ChatChecker int64 `sig:"0A D7 23 3C 00 00 ?? 01"`
	SkinData    int64 `sig:"75 21 8B 1D"`
	Rulesets    int64 `sig:"7D 15 A1 ?? ?? ?? ?? 85 C0"`
	ChatArea    int64 `sig:"33 47 9D FF 5B 7F FF FF"`
*/

use rosu_memory::memory::signature::Signature;

//pub struct

pub struct StaticSignatures {
    pub base: Signature,
    pub menu_mods: Signature,
    pub play_time: Signature,
    pub chat_cheker: Signature,
    pub skin_data: Signature,
    pub rulesets: Signature,
    pub chat_area: Signature,
    pub audio_time_base: Signature,
}

//impl StaticSignatures {
    //fn read
//}
