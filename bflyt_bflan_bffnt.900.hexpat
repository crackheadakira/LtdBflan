/* By Watertoon */
/* Additional modifications by Morgan  */
#pragma pattern_limit 1000000

#include <std/io.pat>
#include <std/mem.pat>
#include <std/core.pat>

u32 cBflytGroupNameLength    = 0x21;
u32 cBflytPaneNameLength     = 0x18;
u32 cBflytUserNameLength     = 0x8;
u32 cBflytMaterialNameLength = 0x1c;

struct Color4u8 {
    u8 r;
    u8 g;
    u8 b;
    u8 a;
};

struct Color4f {
    float r;
    float g;
    float b;
    float a;
};

fn TCharCode32(str string) {
    u32 magic = (string[0] << 0x0) + (string[1] << 0x8) + (string[2] << 0x10) + (string[3] << 0x18);
    return magic;
};

enum BflytSectionType : u32 {
    UserData     = TCharCode32("usd1"),
    Layout       = TCharCode32("lyt1"),
    TextureList  = TCharCode32("txl1"),
    FontList     = TCharCode32("fnl1"),
    MaterialList = TCharCode32("mat1"),
    CaptureTextureList = TCharCode32("ctl1"),
    VectorGraphicsList = TCharCode32("vgl1"),
    PaneStart    = TCharCode32("pas1"),
    PaneEnd      = TCharCode32("pae1"),
    Pane         = TCharCode32("pan1"),
    PicturePane  = TCharCode32("pic1"),
    TextBoxPane  = TCharCode32("txt1"),
    WindowPane   = TCharCode32("wnd1"),
    PartsPane     = TCharCode32("prt1"),
    AlignmentPane = TCharCode32("ali1"),
    CapturePane   = TCharCode32("cpt1"),
    BoundingPane  = TCharCode32("bnd1"),
    ScissorPane   = TCharCode32("scr1"),
    GroupStart    = TCharCode32("grs1"),
    GroupEnd      = TCharCode32("gre1"),
    Group         = TCharCode32("grp1"),
    ControlSource = TCharCode32("cnt1"),
    ShapeInfo = TCharCode32("spi1"),
};

/* User data section */
enum Ui2dLayoutSystemDataType : u32 {
    AnimTagName = 0,
};
enum Ui2dPaneSystemDataType : u32 {
    VertexPos0       = 0,
    VertexPos1       = 1,
    Alignment        = 2,
    MaskTexture      = 3,
    DropShadow       = 4,
    ProceduralShape  = 6,
};
u32 g_last_section_type_for_user_data = 0;

u32 g_system_data_base = 0;
struct ResUi2dSystemDataStringOffset {
    u32 string_offset;
    
    char o_string[] @ g_system_data_base + string_offset;
};

bitfield BflytAlignmentFlags {
    is_margin_enabled : 1;
    is_ignore : 1;
    is_null_pane : 1;
    reserve0 : 29;
};
struct ResUi2dSystemDataAlignment {
    BflytAlignmentFlags options;
    float margin;
};
bitfield BflytProceduralShapeFlags {
    is_color : 1;
    is_inner_shadow : 1;
    is_color_overlay : 1;
    is_gradation_overlay : 1;
    is_drop_shadow : 1;
    is_dual_drop_shadow : 1;
    is_rounded_corner_per_vertex : 1;
    is_drop_shadow_only : 1;
};
struct ResUi2dSystemDataProceduralShape {
    BflytProceduralShapeFlags options;
    u8 color0_options;
    u8 inner_shadow_options;
    u8 inner_shadow_base_comp;
    u8 color_overlay_options;
    u8 gradation_overlay_options;
    u8 drop_shadow_blend_mode;
    u8 drop_shadow_base_comp;
    u32 reserve0[4];
    float rounded_corner0[4];
    float rounded_corner1[4];
    float reserve1;
    Color4f color0;
    Color4f inner_shadow_color;
    float inner_shadow_transform[3];
    Color4f color_overlay;
    float gradation_weights[4];
    Color4f gradation_color_array[4];
    float gradation_rotation;
    Color4f drop_shadow_color;
    float drop_shadow_transform[3];
    u32 reserve2[4];
};

enum TexWrapMode : u8 {
    Clamp,
    Repeat,
    Mirror,
};

enum TexFilter : u8 {
    Near,
    Linear,
};

bitfield TexOptions {
    TexWrapMode wrap_mode : 2;
    TexFilter filter_mode : 6;
};

bitfield DropShadowFlags {
    is_stroke_enabled : 1;
    is_outer_glow_enabled : 1;
    is_drop_shadow_enabled : 1;
    is_knockout : 1;
    is_only_effect : 1;
    is_static_rendering : 1;
    is_degamma_enabled : 1;
    reserve : 1;
};

enum DropShadowBlendMode : u8 {
    Normal,
    Multiply,
    Addition,
    Subtraction,
    NormalMaxAlpha,
};

struct ResUi2dSystemDataDropShadow {
    u16 texture_id;
    TexOptions u_options;
    TexOptions v_options;
    DropShadowFlags flags;
    u24 reserve0;
    
    u8 max_size;
    DropShadowBlendMode stroke_blend_mode;
    DropShadowBlendMode outer_glow_blend_mode;
    DropShadowBlendMode drop_shadow_blend_mode;
    
    u32 reserve5[4];
    
    float stroke_size;
    Color4f stroke_color;
    
    Color4f outer_glow_color;
    float outer_glow_spread;
    float outer_glow_size;
    
    Color4f drop_shadow_color;
    float drop_shadow_angle;
    float drop_shadow_distance;
    float drop_shadow_spread;
    float drop_shadow_size;
    
    u32 reserve15;
    u32 reserve16;
    u32 reserve17;
    u32 reserve18;
};

struct ResUi2dSystemDataMaskTexture {
    u8 flags;
    u24 reserve0;
    u16 texture_id;
    u8 u_options;
    u8 v_options;
    u32 tex_ext_flags;
    u16 capture_texture_id;
    u8 capture_u_options;
    u8 capture_v_options;
    u8 is_use_capture_mask;
    u24 reserve1;
    float translation[2];
    float rotation;
    float scale[2];
};

struct ResUi2dSystemData {
    g_system_data_base = addressof(this);

    if (g_last_section_type_for_user_data == 0) {
        Ui2dLayoutSystemDataType layout_system_data_type;
    
        if (layout_system_data_type == Ui2dLayoutSystemDataType::AnimTagName) {
            u32 string_count;
            ResUi2dSystemDataStringOffset string_offset_array[string_count];
        }
    } else {
        Ui2dPaneSystemDataType pane_system_data_type;
        
        if (pane_system_data_type == Ui2dPaneSystemDataType::VertexPos0 || pane_system_data_type == Ui2dPaneSystemDataType::VertexPos1) {
            float size_scale_width;
            float size_scale_height;
            float position_x_scale;
            float position_y_scale;
        } else if (pane_system_data_type == Ui2dPaneSystemDataType::ProceduralShape) {
            ResUi2dSystemDataProceduralShape proc_shape;
        } else if (pane_system_data_type == Ui2dPaneSystemDataType::Alignment) {
            ResUi2dSystemDataAlignment alignment;
        } else if (pane_system_data_type == Ui2dPaneSystemDataType::DropShadow) {
            ResUi2dSystemDataDropShadow drop_shadow;
        } else if (pane_system_data_type == Ui2dPaneSystemDataType::MaskTexture) {
            ResUi2dSystemDataMaskTexture mask_texture;
        }
    }
};

struct ResUi2dSystemDataArray {
    u16 reserve0;
    u16 count;
    u32 offset;
    
    if (count > 1) {
        u32 section_size;
    }
    
    ResUi2dSystemData o_system_data[count] @ addressof(this) + offset;
};

enum Ui2dUserDataType : u8 {
    String     = 0,
    S32        = 1,
    Float      = 2,
    SystemData = 3,
};

struct ResUi2dUserData {
    u32 name_offset;
    u32 data_array_offset;
    u16 data_count;
    Ui2dUserDataType data_type;
    u8  reserve0;
    
    if (data_type == Ui2dUserDataType::Float) {
        float o_data_array[data_count] @ addressof(this) + data_array_offset;
    } else if (data_type == Ui2dUserDataType::S32) {
        s32 o_data_array[data_count] @ addressof(this) + data_array_offset;
    } else if (data_type == Ui2dUserDataType::String) {
        char o_data_array[data_count] @ addressof(this) + data_array_offset;
    } else if (data_type == Ui2dUserDataType::SystemData) {
        ResUi2dSystemDataArray o_data_array[data_count] @ addressof(this) + data_array_offset;
    }
    
    char o_name[] @ addressof(this) + name_offset;
};

struct ResUi2dUserDataSection {
    u16 user_data_count;
    u16 reserve0;
    ResUi2dUserData user_data_array[user_data_count];
    
    u32 c_real_size = 0x4 + 0xc * user_data_count;
};

/* Layout */
using ResUi2dSectionForEmbed;
struct ResBflytLayout {
    u8    is_centered;
    u8    reserve0;
    u16   reserve1;
    float width;
    float height;
    float parts_width;
    float parts_height;
    char  name[];
    
    u32 c_real_size = 0x14 + sizeof(name);
};


/* Ui2d Binary field layout animation (Bflan) */
enum BflanSectionType : u32 {
    PaneAnimInfo  = TCharCode32("pai1"),
    PaneAnimShare = TCharCode32("pah1"),
    PaneAnimTag   = TCharCode32("pat1"),
};

enum BflanAnimInfoType : u32 {
    PerCharacterTransformCurveAnim = TCharCode32("FLCC"),
    ExtendedUserDataAnim           = TCharCode32("FLEU"),

    PerCharacterTransformAnim      = TCharCode32("FLCT"),
    PaneSrtAnim                    = TCharCode32("FLPA"),
    VertexColorAnim                = TCharCode32("FLVC"),
    VisibilityAnim                 = TCharCode32("FLVI"),
    DropShadowAnim                 = TCharCode32("FLDS"),
    MaskTextureAnim                = TCharCode32("FLMT"),
    ProceduralShapeAnim            = TCharCode32("FLPS"),
    WindowAnim                     = TCharCode32("FLWN"),
    StateMachineAnim               = TCharCode32("FSMA"),

    AlphaCompareAnim               = TCharCode32("FLAC"),
    FontShadowAnim                 = TCharCode32("FLFS"),
    IndirectSrtAnim                = TCharCode32("FLIM"),
    MaterialColorAnim              = TCharCode32("FLMC"),
    TextureSrtAnim                 = TCharCode32("FLTS"),
    TexturePatternAnim             = TCharCode32("FLTP"),
    BrickRepeatAnim                = TCharCode32("FTBR"),
    VectorGraphicsAnim             = TCharCode32("FVGA"),
};

enum BflanPerCharacterTransformCurveTarget : u8 {
    TranslateX      = 0,
    TranslateY      = 1,
    TranslateZ      = 2,
    RotateX         = 3,
    RotateY         = 4,
    RotateZ         = 5,
    LeftTopRed      = 6,
    LeftTopGreen    = 7,
    LeftTopBlue     = 8,
    LeftTopAlpha    = 9,
    LeftBottomRed   = 10,
    LeftBottomGreen = 11,
    LeftBottomBlue  = 12,
    LeftBottomAlpha = 13,
    ScaleX          = 14,
    ScaleY          = 15,
};

enum BflanPerCharacterTransformTarget : u8 {
    EvalTypeOffset = 0,
};
enum BflanPaneSrtTarget : u8 {
    TranslateX = 0,
    TranslateY = 1,
    TranslateZ = 2,
    RotateX    = 3,
    RotateY    = 4,
    RotateZ    = 5,
    ScaleX     = 6,
    ScaleY     = 7,
    SizeX      = 8,
    SizeY      = 9,
};
enum BflanVertexColorTarget : u8 {
   LeftTopRed       = 0,
   LeftTopGreen     = 1,
   LeftTopBlue      = 2,
   LeftTopAlpha     = 3,
   RightTopRed      = 4,
   RightTopGreen    = 5,
   RightTopBlue     = 6,
   RightTopAlpha    = 7,
   LeftBottomRed    = 8,
   LeftBottomGreen  = 9,
   LeftBottomBlue   = 10,
   LeftBottomAlpha  = 11,
   RightBottomRed   = 12,
   RightBottomGreen = 13,
   RightBottomBlue  = 14,
   RightBottomAlpha = 15,
   PaneAlpha        = 16,
};
enum BflanVisibilityTarget : u8 {
    Visibility = 0,
};
enum BflanMaterialColorTarget : u8 {
    BufferRed      = 0,
    BufferGreen    = 1,
    BufferBlue     = 2,
    BufferAlpha    = 3,
    Constant0Red   = 4,
    Constant0Green = 5,
    Constant0Blue  = 6,
    Constant0Alpha = 7,
    Color0Red      = 8,
    Color0Green    = 9,
    Color0Blue     = 10,
    Color0Alpha    = 11,
    Color1Red      = 12,
    Color1Green    = 13,
    Color1Blue     = 14,
    Color1Alpha    = 15,
    Color2Red      = 16,
    Color2Green    = 17,
    Color2Blue     = 18,
    Color2Alpha    = 19,
    Color3Red      = 20,
    Color3Green    = 21,
    Color3Blue     = 22,
    Color3Alpha    = 23,
    Color4Red      = 24,
    Color4Green    = 25,
    Color4Blue     = 26,
    Color4Alpha    = 27,
};
enum BflanTextureSrtTarget : u8 {
    TranslateU = 0,
    TranslateV = 1,
    Rotate     = 2,
    ScaleU     = 3,
    ScaleV     = 4,
};
enum BflanTexturePatternTarget : u8 {
    Image = 0,
};
enum BflanAlphaCompareTarget : u8 {
    CompareReference = 0,
};
enum BflanFontShadowTarget : u8 {
    BlackRed   = 0,
    BlackGreen = 1,
    BlackBlue  = 2,
    WhiteRed   = 3,
    WhiteGreen = 4,
    WhiteBlue  = 5,
    WhiteAlpha = 6,
};

enum BflanCurveType : u8 {
    Constant = 0,
    Step     = 1,
    Hermite  = 2,
};

struct ResBflanHermiteKey {
    float key_index;
    float value;
    float slope;
};
struct ResBflanStepKey {
    float key_index;
    u16   value;
    u16   reserve0;
};

u32 s_last_anim_info_magic = 0;
struct ResBflanAnimTarget {
    u8  layer;
    
    if (s_last_anim_info_magic == BflanAnimInfoType::PerCharacterTransformCurveAnim) {
        BflanPerCharacterTransformCurveTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::PerCharacterTransformAnim) {
        BflanPerCharacterTransformTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::PaneSrtAnim) {
        BflanPaneSrtTarget target_index;
    }  else if (s_last_anim_info_magic == BflanAnimInfoType::VertexColorAnim) {
        BflanVertexColorTarget target_index;
    }  else if (s_last_anim_info_magic == BflanAnimInfoType::VisibilityAnim) {
        BflanVisibilityTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::MaterialColorAnim) {
        BflanMaterialColorTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::TextureSrtAnim) {
        BflanTextureSrtTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::TexturePatternAnim) {
        BflanTexturePatternTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::AlphaCompareAnim) {
        BflanAlphaCompareTarget target_index;
    } else if (s_last_anim_info_magic == BflanAnimInfoType::FontShadowAnim) {
        BflanFontShadowTarget target_index;
    } else {
        u8  target_index;
    }
    
    BflanCurveType  curve_type;
    u8  reserve1;
    u16 frame_count;
    u16 reserve2;
    u32 key_array_offset;
    
    if (curve_type == BflanCurveType::Constant) {
        float o_constant_key_array[frame_count] @ addressof(this) + key_array_offset;
    } else if (curve_type == BflanCurveType::Step) {
        ResBflanStepKey o_step_key_array[frame_count] @ addressof(this) + key_array_offset;
    } else if (curve_type == BflanCurveType::Hermite) {
        ResBflanHermiteKey o_hermite_key_array[frame_count] @ addressof(this) + key_array_offset;
    }
};

u32 s_anim_info_base = 0;
struct ResBflanAnimTargetOffset {
    u32 offset;
    
    ResBflanAnimTarget o_anim_target @ s_anim_info_base + offset [[inline]];
};

struct ResBflanAnimInfo {
    BflanAnimInfoType magic;
    u8  anim_target_count;
    u8  reserve0;
    u16 reserve1;
    
    s_anim_info_base = addressof(this);
    s_last_anim_info_magic = u32(magic);
    
    ResBflanAnimTargetOffset anim_target_offset_array[anim_target_count];
};

u32 s_anim_content_base = 0;
struct ResBflanAnimInfoOffset {
    u32 offset;
    
    ResBflanAnimInfo o_anim_info @ s_anim_content_base + offset [[inline]];
};

enum BflanAnimType : u8 {
    Pane         = 0,
    Material     = 1,
    User         = 2,
    PaneExt      = 3,
    StateMachine = 4,
};

struct ResBflanAnimContent {
    char          anim_name[0x1c];
    u8            anim_info_count;
    BflanAnimType anim_type;
    u16           reserve0;

    if (anim_type == BflanAnimType::User) {
        u32 info_array_offset;
        u32 total_size;
    }

    s_anim_content_base = addressof(this);
    
    ResBflanAnimInfoOffset anim_info_offset_array[anim_info_count];
};

u32 s_pane_anim_info_base = 0;
struct ResBflanAnimContentOffset {
    u32 offset;
    
    ResBflanAnimContent o_anim_content @ s_pane_anim_info_base + offset [[inline]];
};

struct ResBflanPaneAnimInfo {
    u16 frame_count;
    u8  is_looping;
    u8  reserve0;
    u16 texture_count;
    u16 anim_content_count;
    u32 anim_content_offset_array_offset;
    u32 texture_name_offset_array[texture_count];
    
    s_pane_anim_info_base = addressof(this) - 0x8;
    
    ResBflanAnimContentOffset o_anim_content_array[anim_content_count] @ s_pane_anim_info_base + anim_content_offset_array_offset;
    
    u32 c_real_size = 0xc + 0x4 * texture_count;
};

struct ResBflanPaneAnimShare {
    u32 share_info_offset;
    u16 share_info_count;
    u16 reserve0;
    
    u32 c_real_size = 0x8;
};

struct ResBflanGroup {
    char group_name[0x21];
    u8   flag;
    u16  reserve0;
};

using ResUi2dSection;
struct ResBflanPaneAnimTag {
    u16 tag_order;
    u16 group_count;
    u32 name_offset;
    u32 group_array_offset;
    u32 user_data_section_offset;
    u16 start_frame;
    u16 end_frame;
    u8  is_descending_bind;
    u8  reserve0;
    u16 reserve1;
    
    char o_name[] @ addressof(this) - 0x8 + name_offset;
    
    if (group_count != 0) {
        ResBflanGroup o_group_array[group_count] @ addressof(this) - 0x8 + group_array_offset;
    }
    if (user_data_section_offset != 0) {
        ResUi2dSection o_user_data @ addressof(this) - 0x8 + user_data_section_offset;
    }
    
    u32 c_real_size = 0x18;
};

/* Texture List */
u32 texture_name_offset_helper;

struct ResBflytTextureOffset {
    u32 texture_offset;
    
    char o_texture_name[] @ texture_name_offset_helper + texture_offset;
};
struct ResBflytTextureList {
    u16 texture_count;
    u16 reserve0;
    
    texture_name_offset_helper = addressof(this) + 0x4;
    
    ResBflytTextureOffset name_offset_array[texture_count];

    u32 c_real_size = 0x4 + 0x4 * texture_count;
};

/* Font List */
u32 font_name_offset_helper;

struct ResBflytFontOffset {
    u32 font_offset;
    
    char o_font_name[] @ font_name_offset_helper + font_offset;
};

struct ResBflytFontList {
    u16  font_count;
    u16  reserve0;
    
    font_name_offset_helper = addressof(this) + 0x4;
    
    ResBflytFontOffset  name_offset_array[font_count];

    u32 c_real_size = 0x4 + 0x4 * font_count;
};

/* Material Color data */
enum BflytMaterialColorType : u8 {
    U8    = 0,
    Float = 1
};
bitfield ResBflytMaterialColorDataColorTypes {
    BflytMaterialColorType color_0_type : 1;
    BflytMaterialColorType color_1_type : 1;
    BflytMaterialColorType color_2_type : 1;
    BflytMaterialColorType color_3_type : 1;
    BflytMaterialColorType color_4_type : 1;
    BflytMaterialColorType color_5_type : 1;
    BflytMaterialColorType color_6_type : 1;
    BflytMaterialColorType color_7_type : 1;
};

u32 c_material_color_data_base;
u32 c_color_type_index = 0;
u8  c_color_types_info;
u32 c_color_data_size;
struct ResBflytColorTypeOffset {
    u8 color_offset;

    if ((c_color_types_info >> c_color_type_index) == 0) {
        Color4u8 o_color;
        c_color_data_size += 4;
    } else if ((c_color_types_info >> c_color_type_index) == 1) {
        Color4f o_color;
        c_color_data_size += 0x10;
    }
     
    c_color_data_size  += 1;
    c_color_type_index += 1;
} [[color(std::format("{}{}{}{}",o_color.r,o_color.g,o_color.b,o_color.a))]];

struct ResBflytMaterialColorData {
    u8 color_types;
    u8 color_count;
    
    c_material_color_data_base = addressof(this);
    c_color_type_index         = 0;
    c_color_types_info         = color_types;
    c_color_data_size          = 0x2;
    
    ResBflytColorTypeOffset color_offset_array[color_count];
};

/* Material texture map */
bitfield ResBflytMaterialTextureOptions {
    wrap_mode : 2;
    filter    : 2;
    reserve0  : 4;
};
struct ResBflytMaterialTextureMap {
    u16                            texture_index;
    ResBflytMaterialTextureOptions u_options;
    ResBflytMaterialTextureOptions v_options;
};

/* Material texture extensions */
bitfield ResBflytMaterialTextureExtension {
    is_capture_texture : 1;
    is_vector_texture  : 1;
    reserve0           : 30;
};

/* Material texture srt */
struct ResBflytMaterialTextureSrt {
    float translation_x;
    float translation_y;
    float rotation;
    float scale_x;
    float scale_z;
};

/* Material texture coord gen */
enum BflytTexGenType : u8 {
    Tex0                            = 0,
    Tex1                            = 1,
    Tex2                            = 2,
    OrthogonalProjection            = 3,
    PaneBasedProjection             = 4,
    PerspectiveProjection           = 5,
    PaneBasedPerspectiveProjection  = 6,
    BrickRepeat                     = 7
};

struct ResBflytMaterialTexCoordGen {
    u8  reserve0;
    BflytTexGenType tex_gen_type;
    u16 reserve1;
    u32 reserve2;
    u64 reserve3;
};

/* Material Tev combiner */
enum BflytTevStage : u8 {
    Replace = 0x0,
    Modulate = 0x1,
    Add = 0x2,
    AddSigned = 0x3,
    Interpolate = 0x4,
    Subtract = 0x5,
    AddMultiply = 0x6,
    MultiplyAdd = 0x7,
    Overlay = 0x8,
    Lighten = 0x9,
    Darken = 0xa,
    Indirect = 0xb,
    BlendIndirect = 0xc,
    EachIndirect = 0xd,
};
struct ResBflytMaterialTevCombiner {
    BflytTevStage stage;
    u8 reserve0;
    u8 reserve1;
    u8 reserve2;
};

/* Material alpha compare */
enum BflytAlphaTestFunction : u8 {
    Never         = 0,
    LessThan      = 1,
    LessThanEqual = 2,
    Equal         = 3,
    NotEqual      = 4,
    GreaterEqual  = 5,
    Greater       = 6,
    Always        = 7,
};

enum BflytBlendMode : u8 {
    None = 0,
    Blend = 1,
    Logic = 2,
};

struct ResBflytMaterialAlphaCompare {
    BflytAlphaTestFunction alpha_test_function;
    u8    reserve0;
    u16   reserve1;
    float alpha_compare_ref_value;
};

/* Material blend mode */
/* Default Add */
enum BflytBlendEquation : u8 {
    Invalid    = 0,
    Add        = 1,
    Sub        = 2,
    ReverseSub = 3,
    Min        = 4,
    Max        = 5,
};

/* Default V0 */
enum BflytBlendFunction : u8 {
    V0                       = 0,
    V1_0                     = 1,
    DestinationColor         = 2,
    InvDestinationColor      = 3,
    SourceAlpha              = 4,
    InvSourceAlpha           = 5,
    DestinationAlpha         = 6,
    InvDestinationAlpha      = 7,
    SourceColor              = 8,
    InvSourceColor           = 9
};

/* Default Clear */
enum BflytLogicOp : u8 {
    Invalid      = 0,
    NoOp         = 1,
    Clear        = 2,
    Set          = 3,
    Copy         = 4,
    CopyInverted = 5,
    Invert       = 6,
    And          = 7,
    Nand         = 8,
    Or           = 9,
    Nor          = 10,
    Xor          = 11,
    Equivalent   = 12,
    AndReverse   = 13,
    AndInverted  = 14,
    OrReverse    = 15,
    OrInverted   = 16,
};

struct ResBflytMaterialBlendMode {
    BflytBlendEquation blend_equation;
    BflytBlendFunction blend_function_source;
    BflytBlendFunction blend_function_destination;
    BflytLogicOp       logic_op;
};

struct ResBflytMaterialIndirectMatrix {
    float translation[2];
    float rotation;
};

bitfield ProjectionTexGenFlags {
    fitting_layout_size  : 1;
    fitting_pane_size    : 1; 
    adjust_projection_sr : 1;
    reserve0             : 5;
};

struct ResBflytMaterialProjectionTexGen {
    float translation_x;
    float translation_y;
    float scale_x;
    float scale_y;
    ProjectionTexGenFlags flags;
    u8 reserve0[3];
};

struct ResBflytMaterialFontShadowColor {
    Color4u8 color0;
    Color4u8 color1;
};

struct ResBflytMaterialDetailedCombiner {
    u32 value;
    Color4u8 Color1;
    Color4u8 Color2;
    Color4u8 Color3;
    Color4u8 Color4;
    Color4u8 Color5;
    u32 stage_flags;
};

enum TevSource : u8 {
    Primary = 0,
    Unknown2 = 2,
    Texture0 = 3,
    Texture1 = 4,
    Texture2 = 5,
    Texture3 = 6,
    Register = 13,
    Constant = 14,
    Previous = 15,
};

enum TevColorOp : u8 {
    RGB,
    InvRGB,
    Alpha,
    InvAlpha,
    RRR,
    InvRRR,
    GGG = 8,
    InvGGG = 9,
    BBB = 12,
    InvBBB = 13,
};

enum TevScale : u8 {
    V1,
    V2,
    V4,
    Unknown,
};

enum TevAlphaOp : u8 {
    Alpha,
    InvAlpha,
    R,
    InvR,
    G,
    InvG,
    B,
    InvB,
    Unknown,
};

enum DetailedCombinerStageMode : u8 {
    Replace         = 0,
    Modulate        = 1,
    Add             = 2,
    AddSigned       = 3,
    Interpolate     = 4,
    Subtract        = 5,
    AddMultiplicate = 8,
    MultiplicateAdd = 9
};

enum TevKonstSel : u8 {
    BlackColor = 0,
    WhiteColor = 1,
    K0         = 2,
    K1         = 3,
    K2         = 4,
    K3         = 5,
    K4         = 6
};

bitfield ColorFlags {
    TevSource source_0             : 4;
    TevSource source_1             : 4;
    TevSource source_2             : 4;
    TevColorOp operand_0           : 4;
    TevColorOp operand_1           : 4;
    TevColorOp operand_2           : 4;
    DetailedCombinerStageMode mode : 4;
    TevScale scale                 : 2;
    bool copy_reg                  : 1;
    padding                        : 1;
};

bitfield AlphaFlags {
    TevSource source_0             : 4;
    TevSource source_1             : 4;
    TevSource source_2             : 4;
    TevAlphaOp operand_0           : 4;
    TevAlphaOp operand_1           : 4;
    TevAlphaOp operand_2           : 4;
    DetailedCombinerStageMode mode : 4;
    TevScale scale                 : 2;
    padding                        : 2;
};

bitfield ConstantSelectors {
    TevKonstSel color_konst_sel    : 4;
    TevKonstSel alpha_konst_sel    : 4;
    padding                        : 24;
};

struct MaterialDetailedCombinerEntry {
    ColorFlags color_flags;
    AlphaFlags alpha_flags;
    ConstantSelectors constant_selectors;
    u32 reserve0;
};

struct ResBflytMaterialUserCombiner {
    char name[0x60];
    u32  reserve[5];
};

struct ResBflytMaterialVectorTextureInfo {
    float    time;
    Color4u8 color;
    u64      reserve0;
};

struct ResBflytMaterialBrickRepeatShaderInfo {
    u32 reserve0[0x16];
};

/* Material */
bitfield ResBflytMatMemCount {
     tex_map_count                          : 2;
     tex_srt_count                          : 2;
     tex_coord_gen_count                    : 2;
     tev_combiner_count                     : 3;
     has_alpha_compare                      : 1;
     has_color_blend_mode                   : 1;
     use_texture_only                       : 1;
     has_separate_blend_mode                : 1;
     reserve1                               : 1;
     indirect_matrix_count                  : 1;
     projection_tex_gen_count               : 2;
     has_font_shadow_parameter              : 1;
     use_thresholding_alpha_interpolation   : 1;
     use_detailed_combiner                  : 1;
     user_combiner_count                    : 1;
     has_texture_extensions                 : 1;
     vector_texture_info_count              : 2;
     brick_repeat_shader_info_count         : 2;
     reserve3                               : 6;
};

struct ResBflytMaterialData {
    char                             material_name[cBflytMaterialNameLength];
    ResBflytMatMemCount              mat_mem;
    ResBflytMaterialColorData        color_data;
    
    ResBflytMaterialTextureMap       tex_map_array[mat_mem.tex_map_count];
    
    if (mat_mem.has_texture_extensions) {
        ResBflytMaterialTextureExtension tex_ext_array[mat_mem.tex_map_count];
    }
    
    ResBflytMaterialTextureSrt       tex_srt_array[mat_mem.tex_srt_count];
    ResBflytMaterialTexCoordGen      tex_coord_gen_array[mat_mem.tex_coord_gen_count];
    ResBflytMaterialTevCombiner      tev_combiner_array[mat_mem.tev_combiner_count];
    
    if (mat_mem.has_alpha_compare > 0) {
        ResBflytMaterialAlphaCompare alpha_compare;
    }
    
    if (mat_mem.has_color_blend_mode != 0) {
        ResBflytMaterialBlendMode        blend_mode;
    }
    
    if (mat_mem.has_separate_blend_mode != 0) {
        ResBflytMaterialBlendMode        blend_mode_alpha;
    }

    if (mat_mem.indirect_matrix_count > 0) {
        ResBflytMaterialIndirectMatrix   indirect_matrix_array[mat_mem.indirect_matrix_count];
    }
    
    if (mat_mem.use_detailed_combiner != 0) {
        ResBflytMaterialDetailedCombiner detailed_combiner;
        MaterialDetailedCombinerEntry detailed_combiner_entries[mat_mem.tev_combiner_count];
    }
 
    if (mat_mem.projection_tex_gen_count > 0) {
        ResBflytMaterialProjectionTexGen projection_tex_gen_array[mat_mem.projection_tex_gen_count];
    }
    
    if (mat_mem.has_font_shadow_parameter > 0) {
        ResBflytMaterialFontShadowColor  font_shadow_color_array[mat_mem.font_shadow_color];
    }
    
    if (mat_mem.user_combiner_count > 0) {
        ResBflytMaterialUserCombiner     user_combiner_array[mat_mem.user_combiner_count];
    }
    
    if (mat_mem.vector_texture_info_count > 0) {
        ResBflytMaterialVectorTextureInfo vector_texture_info_array[mat_mem.vector_texture_info_count];
    }
    
    if (mat_mem.brick_repeat_shader_info_count > 0) {
        ResBflytMaterialBrickRepeatShaderInfo brick_repeat_gen_array[mat_mem.brick_repeat_shader_info_count];
    }
};

u32 material_list_base;
struct ResBflytMaterialDataOffset {
    u32 material_data_offset;
    
    ResBflytMaterialData o_material_data @ material_list_base + material_data_offset;
};

struct ResBflytMaterialList {
    u16 material_count;
    u16 reserve0;
    
    material_list_base = addressof(this) - 0x8;
    
    ResBflytMaterialDataOffset material_data_offsets[material_count];
    
    u32 c_real_size = 0x4 + 0x4 * material_count;
};

/* Capture texture list */
u32 s_capture_texture_list_base = 0;

struct ResBflytCapturePaneInfo {
    u32   pane_name_offset0;
    u32   pane_name_offset1;
    u32   reserve0[6];
    u8    values[8];
    float reserve1;
    float reserve2;
    
    char o_pane0_name[] @ s_capture_texture_list_base + pane_name_offset0;
    char o_pane1_name[] @ s_capture_texture_list_base + pane_name_offset1;
};
struct ResBflytCaptureTextureList {
    s_capture_texture_list_base = addressof(this) - 0x8;

    u32                     capture_info_count;
    ResBflytCapturePaneInfo capture_info_array[capture_info_count];
    
    u32 c_real_size = 0x4 + 0x30 * capture_info_count;
    
};

/* Vector graphics list */
u32 s_vector_graphics_list_base = 0;

struct ResBflytVectorGraphicsInfo {
    u32  reserve1;
    u32  reserve2;
    u32  reserve3;
    char bnvg_name[];
};

struct ResBflytVectorGraphicsInfoOffset {
    u32 offset;
    
    ResBflytVectorGraphicsInfo o_graphics_info @ s_vector_graphics_list_base + offset;
};

struct ResBflytVectorGraphicsList {
    s_vector_graphics_list_base = addressof(this) - 8;

    u32                              vector_graphics_count;
    ResBflytVectorGraphicsInfoOffset vector_graphics_info_array[vector_graphics_count];
    
    u32 c_real_size = 0x4 + 0x4 * vector_graphics_count;
};

/* Pane */
bitfield BflytPaneFlags {
    is_visible : 1;
    is_scale_child_alpha : 1;
    reserve0 : 6;
};
bitfield BflytPaneFlagsEx {
    is_no_scale_by_parts : 1;
    is_scale_size_by_parts_root : 1;
    is_ext_user_data : 1;
    reserve0 : 5;
};
enum BflytOrigin : u8 {
    Center,
    Left_Top,
    Right_Bottom,
};
enum BflytParentOrigin : u8 {
    None,
    Left_Top,
    Right_Bottom,
};
bitfield BflytOriginBitField {
    BflytOrigin origin_x : 2;
    BflytOrigin origin_y : 2;
    BflytParentOrigin parent_origin_x : 2;
    BflytParentOrigin parent_origin_y : 2;
};
struct ResBflytPane {
    BflytPaneFlags      pane_flags;
    BflytOriginBitField origin;
    u8                  alpha;
    BflytPaneFlagsEx    flag_ex;
    char                pane_name[cBflytPaneNameLength];
    char                user_name[cBflytUserNameLength];
    float               translation_x;
    float               translation_y;
    float               translation_z;
    float               rotation_x;
    float               rotation_y;
    float               rotation_z;
    float               scale_x;
    float               scale_y;
    float               size_x;
    float               size_y;

    u32 c_real_size = 0x4c;
};

/* Picture pane */
struct ResBflytTextureUv {
    float top_left_x;
    float top_left_y;
    float top_right_x;
    float top_right_y;
    float bottom_left_x;
    float bottom_left_y;
    float bottom_right_x;
    float bottom_right_y;
};

struct ResBflytPicturePane : ResBflytPane {
    Color4u8          top_left_vertex_color;
    Color4u8          top_right_vertex_color;
    Color4u8          bottom_left_vertex_color;
    Color4u8          bottom_right_vertex_color;
    u16               material_index;
    u8                texture_count;
    u8                is_shape;
    ResBflytTextureUv texture_uv_array[texture_count];
    
    if (is_shape == 1) {
        u32 shape_info_array_index;
    }

    c_real_size = c_real_size + 0x14 + 0x20 * texture_count;
};

/* Text box pane */
struct ResBflytPerCharacterTransform {
    float eval_time_offset;
    float eval_time_width;
    u8    loop_type;
    u8    origin_v;
    u8    has_anim_info;
    u8    reserve0;
    u8    char_info[20];
    
    if (has_anim_info != 0) {
        ResBflanAnimInfo anim_info;
        s_last_anim_info_magic = anim_info.magic;
        ResBflanAnimTarget anim_info;
    }
};

bitfield BflytTextOriginBitField {
    text_origin_x : 2;
    text_origin_y : 2;
    reserve0      : 4;
};
bitfield BflytTextPaneFlags {
    is_enable_shadow : 1;
    is_limit_glyph_count_to_length : 1;
    is_invisible_border : 1;
    is_double_border : 1;
    is_per_character_transform : 1;
    is_enable_center_ceiling : 1;
    is_line_transform : 1;
    is_default_tag_processor : 1;
    is_per_character_transform_split_by_char_width    : 1;
    is_mix_shadow_alpha : 1;
    is_reverse : 1;
    is_per_character_transform_origin_to_center    : 1;
    reserve0 : 1;
    reserve1 : 1;
    reserve2 : 2;
};

struct ResBflytTextBoxPane : ResBflytPane {
    u16                     text_buffer_size;
    u16                     text_length;
    u16                     material_index;
    u16                     font_index;
    BflytTextOriginBitField text_origin;
    u8                      line_alignment;
    BflytTextPaneFlags      text_flags;
    float          italic_tilt;
    u32            text_offset;
    Color4u8       font_top_color;
    Color4u8       font_bottom_color;
    float          font_size_x;
    float          font_size_y;
    float          character_space;
    float          line_space;
    u32            label_offset;
    float          shadow_translation_x;
    float          shadow_translation_y;
    float          shadow_size_x;
    float          shadow_size_y;
    Color4u8       shadow_top_color;
    Color4u8       shadow_bottom_color;
    float          shadow_italic_tilt;
    u32            line_transform_offset;
    u32            per_character_transform_offset;

    if (text_offset != 0) {
        char16 o_text[text_length >> 1] @ addressof(this) + text_offset - 0x8;
    }
    
    if (label_offset != 0) {
        char o_label[] @ addressof(this) + label_offset - 0x8;
    }
    
    if (text_flags.is_per_character_transform) {
        ResBflytPerCharacterTransform per_character_transform @ addressof(this) + per_character_transform_offset - 0x8;
    }

    c_real_size = c_real_size + 0x54;
};

enum ShapeInfoType : u16 {
    NormalQuad              = 0,
    PrimitiveRoundRect      = 1,
    PrimitiveCircle         = 2,
};

struct ResShapeInfoGfxPrimitiveRoundRect {
    u32 radius;
    u32 slice;
};

struct ResShapeInfoGfxPrimitiveCircle {
    u32 slice;
};

struct ResBflytShapeInfo {
    ShapeInfoType type;
    u8 reserve0[2];
    
    if (type == ShapeInfoType::PrimitiveRoundRect) {
        ResShapeInfoGfxPrimitiveRoundRect round_rect;
    } else if (type == ShapeInfoType::PrimitiveCircle) {
        ResShapeInfoGfxPrimitiveCircle circle;
    }
};

struct ResShapeInfoList {
    u16 count;
    u8 reserve0[2];
    u32 data_offset;
    
    ResBflytShapeInfo shapes[count];
};

/* Window pane */
struct Vector2f {
    float x;
    float y;
};

struct ResBflytWindowContent {
    Color4u8 top_left_vertex_color;
    Color4u8 top_right_vertex_color;
    Color4u8 bottom_left_vertex_color;
    Color4u8 bottom_right_vertex_color;
    u16      material_index;
    u8       uv_coordinate_count;
    u8       reserve0;
    
    ResBflytTextureUv uv_sets[uv_coordinate_count];
};

enum BflytTextureFlipMode : u8 {
    None      = 0,
    FlipU     = 1,
    FlipV     = 2,
    Rotate90  = 3,
    Rotate180 = 4,
    Rotate270 = 5,
};
struct ResBflytWindowFrame {
    u16                  material_index;
    BflytTextureFlipMode texture_flip_mode;
    u8                   reserve0;
};

u32 c_wnd_pane_base;
struct ResBflytWindowFrameOffset {
    u32 frame_offset;
    
    ResBflytWindowFrame o_window_frame @ c_wnd_pane_base + frame_offset;
};

enum WindowKind : u8 {
    Around,
    Horizontal,
    HorizontalNoContent,
};

bitfield ResBflytWindowPaneFlags {
    use_left_corner_material : 1;
    use_vertex_color_for_all_window : 1;
    WindowKind window_kind : 2;
    not_draw_content : 1;
    reserve : 3;
};

struct ResBflytWindowPane : ResBflytPane {
    s16 inflation_left;
    s16 inflation_right;
    s16 inflation_top;
    s16 inflation_bottom;
    s16 frame_size_left;
    s16 frame_size_right;
    s16 frame_size_top;
    s16 frame_size_bottom;
    u8  frame_count;
    ResBflytWindowPaneFlags  flag;
    u16 reserve0;
    u32 content_offset;
    u32 frame_offset_array_offset;

    c_wnd_pane_base = addressof(this) - 0x8;

    ResBflytWindowContent     o_content @ c_wnd_pane_base + content_offset;
    ResBflytWindowFrameOffset o_frame_offset_array[frame_count] @ c_wnd_pane_base + frame_offset_array_offset;

    c_real_size = c_real_size + 0x1c;
};

/* Part pane */
struct ResBflytPartsPaneBasicInfo {
    char  user_name[cBflytUserNameLength];
    float translation_x;
    float translation_y;
    float translation_z;
    float rotation_x;
    float rotation_y;
    float rotation_z;
    float scale_x;
    float scale_y;
    float size_x;
    float size_y;
    u8    pane_alpha;
    u8    reserve0;
    u8    reserve1;
};

u32 g_last_parts_pane = 0;
struct ResBflytPartsProperty {
    char property_name[cBflytPaneNameLength];
    u8   usage_flag;
    u8   basic_usage_flag;
    u8   material_usage_flag;
    u8   user_data_type;
    u32  pane_offset;
    u32  user_data_offset;
    u32  pane_basic_info_offset;

    if (pane_offset != 0) {
        ResUi2dSectionForEmbed o_section @ g_last_parts_pane + pane_offset;
    }
    if (user_data_offset != 0) {
        ResUi2dSectionForEmbed o_user_data @ g_last_parts_pane + user_data_offset;
    }
    if (pane_basic_info_offset != 0) {
        ResBflytPartsPaneBasicInfo o_basic_info @ g_last_parts_pane + pane_basic_info_offset;
    }
};

struct ResBflytPartsPane : ResBflytPane {
    u32                   property_count;
    float                 magnify_x;
    float                 magnify_y;

    u32 last_parts = g_last_parts_pane;
    g_last_parts_pane = addressof(this) - 0x8;

    ResBflytPartsProperty property_array[property_count];
    char                  o_layout_name[] @ addressof(this) + c_real_size + 0xc + 0x28 * property_count;

    g_last_parts_pane = last_parts;

    c_real_size = c_real_size + 0xc + 0x28 * property_count;
};

/* Alignment pane */
enum BflytAlignmentDirectionType : u32 {
    Center,
    Forward,
    Reverse,
};
struct ResBflytAlignmentPane : ResBflytPane {
    BflytAlignmentDirectionType direction;
    float default_margin;
    u8 is_align_last_pane;
    u8 is_vertical_alignment;
    u16 reserve0;
};

/* Capture pane */
struct ResBflytCapturePane : ResBflytPane {
    u32 reserve0[4];
    Color4f clear_color;
    u16 image_format;
    u8 is_copy_framebuffer;
    u8 is_create_resource;
    u16 reserve1;
    u24 reserve2;
    u24 reserve3;
    float scale;
};

/* Group */
struct ResBflytPaneName {
    char name[cBflytPaneNameLength];
};
struct ResBflytGroup {
    char group_name[cBflytGroupNameLength];
    u8   reserve0;
    u16  child_count;
    ResBflytPaneName child_name_array[child_count];

    u32 c_real_size = 0x24 + child_count * cBflytPaneNameLength;
};

struct ResBflytStringOffset<auto TableBase> {
    u32 string_offset;
    char value[] @ TableBase + string_offset [[hex::string]];
};

/* Control Source */
struct ResBflytControlSource {
    u32 reserve0_offset;
    u32 name_array_offset;
    u16 pane_count;
    u16 anim_count;
    u32 pane_name_offset_array;
    u32 anim_name_offset_array;
    
    u32 section_base     = addressof(this) - 0x8;
    u32 na_base          = section_base + name_array_offset;
    u32 core_table_base  = na_base + pane_count * cBflytPaneNameLength;
    u32 pane_table_base  = section_base + pane_name_offset_array;
    u32 anim_table_base  = section_base + anim_name_offset_array;
    
    char o_control_name[]  @ addressof(this) + 0x14;
    char o_reserve0_name[] @ section_base + reserve0_offset;
    
    char o_pane_binding_slots[pane_count * cBflytPaneNameLength] @ na_base;
    
    ResBflytStringOffset<core_table_base> o_core_anim_table[anim_count] @ core_table_base;
    
    ResBflytStringOffset<pane_table_base> o_pane_name_table[pane_count] @ pane_table_base;
    ResBflytStringOffset<anim_table_base> o_anim_name_table[anim_count] @ anim_table_base;
};

/* Bffnt */
enum BffntSectionType : u32 {
    FontInformation = TCharCode32("FINF"),
    TextureGlyph = TCharCode32("TGLP"),
    KerningTable = TCharCode32("KRNG"),
    CharWidth = TCharCode32("CWDH"),
    CodeMap   = TCharCode32("CMAP"),
};

enum BffntImageFormat : u8 {
    R8G8B8A8_Unorm = 0x0,
    R8G8B8_Unorm   = 0x1,
    R5G5B5A1_Unorm = 0x2,
    R5G6B5_Unorm   = 0x3,
    R4G4B4A4_Unorm = 0x4,
    LA8_Unorm      = 0x5,
    LA4_Unorm      = 0x6,
    A4_Unorm       = 0x7,
    A8_Unorm       = 0x8,
    BC1_Unorm      = 0x9,
    BC2_Unorm      = 0xa,
    BC3_Unorm      = 0xb,
    BC4_Unorm      = 0xc,
    BC5_Unorm      = 0xd,
    R8G8B8A8_SRGB  = 0xe,
    BC1_SRGB       = 0xf,
    BC2_SRGB       = 0x10,
    BC3_SRGB       = 0x11,
    BC7_Unorm      = 0x12,
    BC7_SRGB       = 0x13,
};

struct ResBffntTextureGlyph {
    u8               cell_width;
    u8               cell_height;
    u8               texture_count;
    u8               max_character_width;
    u32              per_texture_size;
    u16              baseline_position;
    BffntImageFormat image_format;
    u16              cells_per_row;
    u16              cells_per_column;
    u16              image_width;
    u16              image_height;
    u32              image_data_offset;

    u32 c_real_size = 0x18;
};

struct ResBffntCharacterWidthData {
    s8 left_width;
    u8 glyph_width;
    u8 char_width;
};

struct ResBffntCharacterWidth {
    u16                        first_entry_index;
    u16                        last_entry_index;
    u32                        next_character_width;
    ResBffntCharacterWidthData char_width_array[last_entry_index - first_entry_index];

    u32 c_real_size = 0x8 + 3 * last_entry_index - first_entry_index;

    if (next_character_width != 0) {
        ResBffntCharacterWidth o_next @ next_character_width;
    }
};

enum BffntMapMethod : u16 {
    Direct = 0x0,
    Table  = 0x1,
    Scan   = 0x2,
};

struct ResBffntScanEntry {
    u16 key;
    u16 code;
};

struct ResBffntScan {
    u16               scan_half_count;
    u16               reserve0;
    ResBffntScanEntry scan_array[scan_half_count * 2];
};

struct ResBffntCodeMap {
    u32            code_range_begin;
    u32            code_range_end;
    BffntMapMethod map_method;
    u16            reserve0;
    u32            next_code_map_offset;

    u32 c_real_size = 0x10;

    if (map_method == BffntMapMethod::Direct) {
        u16 base_char;

        c_real_size = c_real_size + 2;
    } else if (map_method == BffntMapMethod::Table) {
        u16 mapping_table[code_range_end - code_range_begin];

        c_real_size = c_real_size + 2 * (code_range_end - code_range_begin);
    } else if (map_method == BffntMapMethod::Scan) {
        ResBffntScan o_scan;

        c_real_size += sizeof(o_scan);
    }

    if (next_code_map_offset != 0) {
        ResBffntCodeMap o_next @ next_code_map_offset;
    }
};

enum BffntFontType : u8 {
    Glyph         = 0x0,
    Texture       = 0x1,
    PackedTexture = 0x2
};
enum BffntCharacterEncoding : u8 {
    Unicode  = 0x0,
    ShiftJis = 0x1,
    CP1252   = 0x2
};

struct ResBffntFontInfo {
    BffntFontType          font_type;
    u8                     height;
    u8                     width;
    u8                     ascent;
    u16                    line_feed;
    u16                    alternate_char_index;
    u8                     default_left_width;
    u8                     default_glyph_width;
    u8                     default_character_width;
    BffntCharacterEncoding character_encoding;
    u32                    texture_glyph_offset;
    u32                    character_width_offset;
    u32                    code_map_offset;
    
    u32 c_real_size = 0x18;
    
    if (texture_glyph_offset != 0) {
        ResBffntTextureGlyph o_texture_glyph @ texture_glyph_offset;
    }
    if (character_width_offset != 0) {
        ResBffntCharacterWidth o_character_width @ character_width_offset;
    }
    if (code_map_offset != 0) {
        ResBffntCodeMap o_code_map @ code_map_offset;
    }
};

/* Ui2d headers */
struct ResUi2dSectionForEmbed {
    BflytSectionType magic;
    u32              section_size;
    
    if (magic == BflytSectionType::UserData) {
        ResUi2dUserDataSection user_data;
    } else if (magic == BflytSectionType::Layout) {
        ResBflytLayout layout;
    } else if (magic == BflytSectionType::TextureList) {
        ResBflytTextureList texture_list;
    } else if (magic == BflytSectionType::FontList) {
        ResBflytFontList font_list;
    } else if (magic == BflytSectionType::MaterialList) {
        ResBflytMaterialList material_list;
    } else if (magic == BflytSectionType::CaptureTextureList) {
        ResBflytCaptureTextureList capture_texture_list;
    } else if (magic == BflytSectionType::VectorGraphicsList) {
        ResBflytVectorGraphicsList vector_graphics_list;
    } else if (magic == BflytSectionType::Pane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::PicturePane) {
        ResBflytPicturePane pane;
    } else if (magic == BflytSectionType::TextBoxPane) {
        ResBflytTextBoxPane pane;
    } else if (magic == BflytSectionType::WindowPane) {
        ResBflytWindowPane pane;
    } else if (magic == BflytSectionType::PartsPane) {
        ResBflytPartsPane pane;
    } else if (magic == BflytSectionType::AlignmentPane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::CapturePane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::BoundingPane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::ScissorPane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::Group) {
        ResBflytGroup group;
    } else if (magic == BflytSectionType::ControlSource) {
        ResBflytControlSource control_source;
    } else if (magic == BflanSectionType::PaneAnimShare) {
        ResBflanPaneAnimShare pane_anim_share;
    } else if (magic == BflanSectionType::PaneAnimTag) {
        ResBflanPaneAnimTag pane_anim_tag;
    } else if (magic == BflanSectionType::PaneAnimInfo) {
        ResBflanPaneAnimInfo pane_anim_info;
    } else if (magic == BflytSectionType::ShapeInfo) {
        ResShapeInfoList pane_anim_info;
    }
};

fn LoopUntilPaneEnd() {
    u32 magic @ $;
    if (magic == BflytSectionType::PaneEnd) {
        return false;
    } else {
        g_section_count = g_section_count - 1;
    }
    return true;
};
u32 g_current_section_size;
struct ResUi2dSection {
    BflytSectionType magic;
    u32              section_size;
    
    g_current_section_size = section_size;
    
    if (magic == BflytSectionType::UserData) {
        ResUi2dUserDataSection user_data;
    } else if (magic == BflytSectionType::Layout) {
        ResBflytLayout layout;
        g_last_section_type_for_user_data = 0;
    } else if (magic == BflytSectionType::TextureList) {
        ResBflytTextureList texture_list;
    } else if (magic == BflytSectionType::FontList) {
        ResBflytFontList font_list;
    } else if (magic == BflytSectionType::MaterialList) {
        ResBflytMaterialList material_list;
    } else if (magic == BflytSectionType::CaptureTextureList) {
        ResBflytCaptureTextureList capture_texture_list;
    } else if (magic == BflytSectionType::VectorGraphicsList) {
        ResBflytVectorGraphicsList vector_graphics_list;
    } else if (magic == BflytSectionType::Pane) {
        ResBflytPane pane;
        g_last_section_type_for_user_data = 1;
    } else if (magic == BflytSectionType::PicturePane) {
        ResBflytPicturePane pane;
    } else if (magic == BflytSectionType::TextBoxPane) {
        ResBflytTextBoxPane pane;
    } else if (magic == BflytSectionType::WindowPane) {
        ResBflytWindowPane pane;
    } else if (magic == BflytSectionType::PartsPane) {
        ResBflytPartsPane pane;
    } else if (magic == BflytSectionType::AlignmentPane) {
        ResBflytAlignmentPane pane;
    } else if (magic == BflytSectionType::CapturePane) {
        ResBflytCapturePane pane;
    } else if (magic == BflytSectionType::BoundingPane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::ScissorPane) {
        ResBflytPane pane;
    } else if (magic == BflytSectionType::Group) {
        ResBflytGroup group;
    } else if (magic == BflytSectionType::ControlSource) {
        ResBflytControlSource control_source;
    } else if (magic == BflanSectionType::PaneAnimShare) {
        ResBflanPaneAnimShare pane_anim_share;
    } else if (magic == BflanSectionType::PaneAnimTag) {
        ResBflanPaneAnimTag pane_anim_tag;
    } else if (magic == BflanSectionType::PaneAnimInfo) {
        ResBflanPaneAnimInfo pane_anim_info;
    } else if (magic == BffntSectionType::FontInformation) {
        ResBffntFontInfo font_info [[inline]];
    } else if (magic == BffntSectionType::TextureGlyph) {
       // ResBffntTextureGlyph texture_glyph [[inline]];
    } else if (magic == BffntSectionType::KerningTable) {
    } else if (magic == BffntSectionType::CharWidth) {
        ResBffntCharacterWidth character_width [[inline]];
    } else if (magic == BffntSectionType::CodeMap) {
        ResBffntCodeMap code_map [[inline]];
    } else if (magic == BflytSectionType::ShapeInfo) {
        ResShapeInfoList pane_anim_info;
    }
    
    if (magic == BflytSectionType::PaneStart) {
        ResUi2dSection section_array[while(LoopUntilPaneEnd())];
    } else {
        $ = addressof(this) + section_size;
    }
};

u32 g_section_count = 0;
fn LoopUntilSectionEnd() {
    u32 count = g_section_count;
    g_section_count = g_section_count - 1;
    //std::print("{}", count);
    return (0 < count);
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
    
    g_section_count = section_count;
    ResUi2dSection section_array[while(LoopUntilSectionEnd())] @ addressof(this) + header_size;
};

ResUi2dHeader head @ 0x0;