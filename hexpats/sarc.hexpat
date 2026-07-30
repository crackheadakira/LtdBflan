struct FATEntry {
    u32 filename_hash;
    u32 attributes;
    u32 file_start_offset;
    u32 file_end_offset;
};

struct SFATEntry {
    u32 magic;
    u16 header_size;
    u16 file_count;
    u32 hash_multiplier;
    FATEntry fat[file_count];
};

struct SFNTentry {
    u32 magic;
    u16 header_size;
    u16 pad;
    char string_pool[while($ < parent.offset_to_data)];
};

struct SarcArchive {
    u32 magic;
    u16 header_size;
    u16 endianness;
    u32 file_size;
    u32 offset_to_data;
    u16 version_number;
    u16 pad;
    SFATEntry sfat;
    SFNTentry sfnt;
};

SarcArchive header @ 0x0;

u32 data_section_size = header.file_size - header.offset_to_data;
u8 file_payloads[data_section_size] @ header.offset_to_data [[color("44FF44")]];
