/* By Watertoon */
#pragma pattern_limit 1000000

#include <std/io.pat>
#include <std/mem.pat>
#include <std/core.pat>

enum FcpxDescriptionType : u32 {
    Res = 0,
    Scalable = 1,
    Multi = 2,
};

struct ResFcpxCharRange {
    u32 min;
    u32 max;
};

bitfield FcpxFontFlags {
    is_ignore_proportional_alternate_width : 1;
    is_override_bearing_x : 1;
    is_override_char_width : 1;
    reserve0 : 5;
};

u32 g_last_desc = 0;
struct ResFcpxFont {
    float bold_weight;
    float width_scale;
    u32   font_face_id;
    u32   name_offset;
    u8    outline_width;
    FcpxFontFlags flags;
    u16   baseline_offset;
    float height_scale;
    u16   bearing_x;
    u16   override_char_width;
    u16   ascent;
    u16   descent;
    u32   range_count;
    u32   range_array_offset;
    
    char o_font[] @ g_last_desc + name_offset;
    ResFcpxCharRange o_range_array[range_count] @ g_last_desc + range_array_offset;
};

struct ResFcpxDescription {
    FcpxDescriptionType description_type;

    u32 desc_restore = g_last_desc;
    g_last_desc = addressof(this);

    if (description_type == FcpxDescriptionType::Multi) {
        u32 child_offset;
        u32 next_offset;
        
        ResFcpxDescription o_child @ addressof(this) + child_offset;
        ResFcpxDescription o_next  @ addressof(this) + next_offset;
        
    } else if (description_type == FcpxDescriptionType::Scalable) {
        float width;
        u32   line_feed_delta;
        u32   font_count;
        u32   base_offset;
		u16   replace_char;
        
        ResFcpxFont o_font_array[font_count] @ addressof(this) + base_offset;
    } else if (description_type == FcpxDescriptionType::Res) {
        u32 name_offset;
        u32 range_count;
        u32 range_offset;
        
        char o_font_name[] @ addressof(this) + range_offset;
        ResFcpxCharRange o_range_array[range_count] @ addressof(this) + range_offset;
    }
    
    g_last_desc = desc_restore;
};
                    
struct ResUi2dHeader {
    u32 magic;
    u16 endianess;
    u16 header_size;
    u16 micro_version;
    u8  minor_version;
    u8  major_version;
    u32 file_size;
    u32 section_count;
};

struct ResFcpx : ResUi2dHeader {
    u32 root_offset;
    
    ResFcpxDescription o_root_desc @ addressof(root_offset) + root_offset;
};

ResFcpx head @ 0x0;