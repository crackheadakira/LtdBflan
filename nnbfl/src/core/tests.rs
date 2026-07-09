#[cfg(test)]
mod cursor_tests {
    use crate::core::{Cursor, FormatError};

    fn cur(data: &[u8]) -> Cursor<'_> {
        Cursor {
            data,
            pos: 0,
            ..Default::default()
        }
    }

    #[test]
    fn read_u8_basic() {
        let mut c = cur(&[0xAB]);
        assert_eq!(c.read_u8().unwrap(), 0xAB);
    }

    #[test]
    fn read_u8_eof() {
        let mut c = cur(&[]);
        matches!(
            c.read_u8().unwrap_err(),
            FormatError::UnexpectedEof {
                offset: 0,
                requested_bytes: 1
            }
        );
    }

    #[test]
    fn read_u16_little_endian() {
        let mut c = cur(&[0x01, 0x02]);
        assert_eq!(c.read_u16().unwrap(), 0x0201);
    }

    #[test]
    fn read_u32_little_endian() {
        let mut c = cur(&[0x04, 0x03, 0x02, 0x01]);
        assert_eq!(c.read_u32().unwrap(), 0x01020304);
    }

    #[test]
    fn read_i16_negative() {
        let v: i16 = -300;
        let bytes = v.to_le_bytes();
        let mut c = cur(&bytes);
        assert_eq!(c.read_i16().unwrap(), -300);
    }

    #[test]
    fn read_i32_negative() {
        let v: i32 = -100_000;
        let bytes = v.to_le_bytes();
        let mut c = cur(&bytes);
        assert_eq!(c.read_i32().unwrap(), -100_000);
    }

    #[test]
    fn read_f32_roundtrip() {
        let v: f32 = 3.14159;
        let bytes = v.to_le_bytes();
        let mut c = cur(&bytes);
        assert!((c.read_f32().unwrap() - 3.14159_f32).abs() < 1e-5);
    }

    #[test]
    fn read_fixed_string_with_null_terminator() {
        let data = b"abc\0\0\0\0\0";
        let mut c = cur(data);
        let s = c.read_fixed_string(8).unwrap();

        assert_eq!(s, "abc");
    }

    #[test]
    fn read_fixed_string_no_null_fills_whole_field() {
        let data = b"abcdefgh";
        let mut c = cur(data);
        let s = c.read_fixed_string(8).unwrap();

        assert_eq!(s, "abcdefgh");
    }

    #[test]
    fn read_fixed_string_empty_field() {
        let data = b"\0\0\0\0";
        let mut c = cur(data);

        assert_eq!(c.read_fixed_string(4).unwrap(), "");
    }

    #[test]
    fn read_null_terminated_string_basic() {
        let data = b"hello\0rest";
        let mut c = cur(data);

        assert_eq!(c.read_null_terminated_string().unwrap(), "hello");
        assert_eq!(c.pos, 6);
    }

    #[test]
    fn read_null_terminated_string_no_null() {
        let data = b"neverends";
        let mut c = cur(data);

        assert!(c.read_null_terminated_string().is_err());
    }

    #[test]
    fn read_null_terminated_string_empty() {
        let data = b"\0";
        let mut c = cur(data);

        assert_eq!(c.read_null_terminated_string().unwrap(), "");
    }

    #[test]
    fn seek_within_bounds() {
        let data = [0u8; 16];
        let mut c = cur(&data);
        c.seek(8).unwrap();

        assert_eq!(c.pos, 8);
    }

    #[test]
    fn seek_beyond_eof_errors() {
        let data = [0u8; 4];
        let mut c = cur(&data);

        assert!(c.seek(100).is_err());
    }

    #[test]
    fn seek_relative_advances_pos() {
        let data = [0u8; 16];
        let mut c = cur(&data);

        c.seek_relative(5);
        assert_eq!(c.pos, 5);
    }

    #[test]
    fn pos_advances_correctly() {
        let data: Vec<u8> = (0u8..=9).collect();
        let mut c = cur(&data);

        c.read_u8().unwrap();
        assert_eq!(c.pos, 1);

        c.read_u16().unwrap();
        assert_eq!(c.pos, 3);

        c.read_u32().unwrap();
        assert_eq!(c.pos, 7);
    }
}

#[cfg(test)]
mod writer_tests {
    use crate::core::Writer;

    fn writer() -> Writer {
        Writer::new()
    }

    #[test]
    fn write_u8_appends_single_byte() {
        let mut w = writer();
        w.write_u8(0xFF);

        assert_eq!(w.buffer, &[0xFF]);
    }

    #[test]
    fn write_u16_little_endian() {
        let mut w = writer();
        w.write_u16(0x0102);

        assert_eq!(w.buffer, &[0x02, 0x01]);
    }

    #[test]
    fn write_u32_little_endian() {
        let mut w = writer();
        w.write_u32(0x01020304);

        assert_eq!(w.buffer, &[0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn write_f32_roundtrip() {
        let v: f32 = -1.5;
        let mut w = writer();
        w.write_f32(v);

        assert_eq!(w.buffer, v.to_le_bytes());
    }

    #[test]
    fn write_fixed_string_pads_with_zeros() {
        let mut w = writer();
        w.write_fixed_string("hi", 5);

        assert_eq!(w.buffer, b"hi\0\0\0");
    }

    #[test]
    fn write_fixed_string_truncates_if_too_long() {
        let mut w = writer();
        w.write_fixed_string("abcdef", 3);

        assert_eq!(w.buffer, b"abc");
    }

    #[test]
    fn write_null_terminated_string_appends_null() {
        let mut w = writer();
        w.write_null_terminated_string("test");

        assert_eq!(w.buffer, b"test\0");
    }

    #[test]
    fn patch_u32_overwrites_placeholder() {
        let mut w = writer();
        let pos = w.write_placeholder_u32();

        w.write_u32(0xDEAD_BEEF);
        w.patch_u32(pos, 0xCAFE_BABE);

        assert_eq!(&w.buffer[0..4], 0xCAFE_BABEu32.to_le_bytes());
    }

    #[test]
    fn align_inserts_correct_padding() {
        let mut w = writer();
        w.write_u8(0x01);
        w.align(4);
        assert_eq!(w.pos(), 4);

        w.align(4);
        assert_eq!(w.pos(), 4);
    }

    #[test]
    fn pos_matches_buffer_len() {
        let mut w = writer();
        assert_eq!(w.pos(), 0);

        w.write_u32(1);
        assert_eq!(w.pos(), 4);
    }

    #[test]
    fn mark_records_breadcrumb() {
        let mut w = writer();
        w.write_u8(0);
        w.mark("checkpoint");

        assert_eq!(w.breadcrumbs.len(), 1);
        assert_eq!(w.breadcrumbs[0].0, 1);
        assert_eq!(w.breadcrumbs[0].1, "checkpoint");
    }
}

#[cfg(test)]
mod tchar_tests {
    use crate::core::tchar_code32;

    #[test]
    fn flyt_magic() {
        let magic = tchar_code32(b"FLYT");
        let bytes = magic.to_le_bytes();

        assert_eq!(&bytes, b"FLYT");
    }

    #[test]
    fn flan_magic() {
        let magic = tchar_code32(b"FLAN");
        let bytes = magic.to_le_bytes();

        assert_eq!(&bytes, b"FLAN");
    }

    #[test]
    fn section_magic_roundtrip() {
        for tag in [b"lyt1", b"txl1", b"fnl1", b"mat1", b"pan1", b"grp1"] {
            let magic = tchar_code32(tag);
            let bytes = magic.to_le_bytes();
            assert_eq!(&bytes, tag as &[u8]);
        }
    }
}

#[cfg(test)]
mod flag_tests {
    use crate::{
        bflyt::flags::{BflytPosition, PaneFlagsEx},
        core::BitPackable,
    };

    #[test]
    fn bflyt_origins_roundtrip_all_zeros() {
        let origins = BflytPosition::decode(0x00);

        assert_eq!(origins.encode(), 0x00);
    }

    #[test]
    fn bflyt_origins_roundtrip_all_bits() {
        for o_x in 0u8..3 {
            for o_y in 0u8..3 {
                for p_x in 0u8..3 {
                    for p_y in 0u8..3 {
                        let raw = o_x | (o_y << 2) | (p_x << 4) | (p_y << 6);
                        let decoded = BflytPosition::decode(raw);
                        assert_eq!(decoded.encode(), raw);
                    }
                }
            }
        }
    }
    #[test]
    fn pane_flags_ex_roundtrip() {
        for raw in 0u8..=255 {
            let decoded = PaneFlagsEx::decode(raw);
            assert_eq!(decoded.encode(), raw);
        }
    }
}

#[cfg(test)]
mod bflyt_roundtrip_tests {
    use crate::{
        bflyt::{
            flags::{BflytPosition, PaneFlags, PaneFlagsEx},
            list::{FontList, Layout, TextureList},
            pane::Pane,
        },
        core::{BitPackable, Cursor, ReadWriteable, Writer},
        ui2d::types::{Vector2f, Vector3f},
    };

    fn roundtrip_layout(layout: &Layout) -> Layout {
        let mut w = Writer::new();
        layout.write(&mut w);
        let mut c = Cursor {
            data: &w.buffer,
            pos: 0,
            ..Default::default()
        };
        Layout::parse(&mut c).unwrap()
    }

    #[test]
    fn layout_basic_roundtrip() {
        let layout = Layout {
            is_centered: true,
            width: 1280.0,
            height: 720.0,
            parts_width: 0.0,
            parts_height: 0.0,
            name: "RootLayout".to_string(),
        };

        let rt = roundtrip_layout(&layout);
        assert!(rt.is_centered);
        assert!((rt.width - 1280.0).abs() < 1e-5);
        assert!((rt.height - 720.0).abs() < 1e-5);
        assert_eq!(rt.name, "RootLayout");
    }

    #[test]
    fn layout_not_centered_roundtrip() {
        let layout = Layout {
            is_centered: false,
            width: 640.0,
            height: 480.0,
            parts_width: 320.0,
            parts_height: 240.0,
            name: "SubLayout".to_string(),
        };
        let rt = roundtrip_layout(&layout);
        assert!(!rt.is_centered);
        assert!((rt.parts_width - 320.0).abs() < 1e-5);
    }

    #[test]
    fn layout_empty_name_roundtrip() {
        let layout = Layout {
            is_centered: false,
            width: 0.0,
            height: 0.0,
            parts_width: 0.0,
            parts_height: 0.0,
            name: String::new(),
        };
        let rt = roundtrip_layout(&layout);
        assert_eq!(rt.name, "");
    }

    fn roundtrip_texlist(t: &TextureList) -> TextureList {
        let mut w = Writer::new();
        t.write(&mut w);
        let mut c = Cursor {
            data: &w.buffer,
            pos: 0,
            ..Default::default()
        };

        TextureList::parse(&mut c).unwrap()
    }

    #[test]
    fn texture_list_empty_roundtrip() {
        let tl = TextureList { textures: vec![] };
        let rt = roundtrip_texlist(&tl);

        assert!(rt.textures.is_empty());
    }

    #[test]
    fn texture_list_single_roundtrip() {
        let tl = TextureList {
            textures: vec!["DiffuseTexture".to_string()],
        };
        let rt = roundtrip_texlist(&tl);

        assert_eq!(rt.textures, &["DiffuseTexture"]);
    }

    #[test]
    fn texture_list_multiple_roundtrip() {
        let names = vec![
            "Tex_A".to_string(),
            "Tex_B".to_string(),
            "Tex_C".to_string(),
        ];
        let tl = TextureList {
            textures: names.clone(),
        };
        let rt = roundtrip_texlist(&tl);

        assert_eq!(rt.textures, names);
    }

    fn roundtrip_fontlist(f: &FontList) -> FontList {
        let mut w = Writer::new();
        f.write(&mut w);
        let mut c = Cursor {
            data: &w.buffer,
            pos: 0,
            ..Default::default()
        };
        FontList::parse(&mut c).unwrap()
    }

    #[test]
    fn font_list_empty_roundtrip() {
        let fl = FontList { fonts: vec![] };
        let rt = roundtrip_fontlist(&fl);

        assert!(rt.fonts.is_empty());
    }

    #[test]
    fn font_list_single_roundtrip() {
        let fl = FontList {
            fonts: vec!["NintendoStandard".to_string()],
        };
        let rt = roundtrip_fontlist(&fl);

        assert_eq!(rt.fonts, &["NintendoStandard"]);
    }

    fn make_pane(name: &str) -> Pane {
        Pane {
            pane_flags: PaneFlags {
                is_visible: true,
                influenced_alpha: false,
                location_adjust: false,
                user_allocated: false,
                is_global_matrix_dirty: false,
                is_srt_matrix_user: false,
                is_global_matrix_user: false,
                is_constant_buffer_ready: false,
            },
            position: BflytPosition::decode(0x00),
            alpha: 255,
            flag_ex: PaneFlagsEx::decode(0x00),
            pane_name: name.to_string(),
            user_name: String::new(),
            translation: Vector3f {
                x: 1.0,
                y: 2.0,
                z: 0.0,
            },
            rotation: Vector3f {
                x: 0.0,
                y: 0.0,
                z: 45.0,
            },
            scale: Vector2f { x: 1.0, y: 1.0 },
            size: Vector2f { x: 100.0, y: 50.0 },
        }
    }

    fn roundtrip_pane(p: &Pane) -> Pane {
        let mut w = Writer::new();
        p.write(&mut w);
        let mut c = Cursor {
            data: &w.buffer,
            pos: 0,
            ..Default::default()
        };
        Pane::parse(&mut c).unwrap()
    }

    #[test]
    fn pane_basic_roundtrip() {
        let p = make_pane("RootPane");
        let rt = roundtrip_pane(&p);

        assert_eq!(rt.pane_name, "RootPane");
        assert_eq!(rt.alpha, 255);
        assert!(rt.pane_flags.is_visible);
        assert!((rt.translation.x - 1.0).abs() < 1e-5);
        assert!((rt.size.x - 100.0).abs() < 1e-5);
    }

    #[test]
    fn pane_name_truncated_to_field_length() {
        let long_name = "A".repeat(100);
        let p = make_pane(&long_name);
        let rt = roundtrip_pane(&p);

        assert_eq!(rt.pane_name.len(), 24);
    }

    #[test]
    fn pane_invisible_roundtrip() {
        let mut p = make_pane("HiddenPane");
        p.pane_flags = PaneFlags {
            is_visible: false,
            influenced_alpha: false,
            location_adjust: false,
            user_allocated: false,
            is_global_matrix_dirty: false,
            is_srt_matrix_user: false,
            is_global_matrix_user: false,
            is_constant_buffer_ready: false,
        };
        let rt = roundtrip_pane(&p);
        assert!(!rt.pane_flags.is_visible);
    }
}

#[cfg(test)]
mod bflyt_file_tests {
    use crate::{
        bflyt::file::Bflyt,
        core::{FileReadWriteable, FormatError},
    };

    #[test]
    fn parse_empty_bytes_errors() {
        let result = Bflyt::parse_file(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_wrong_magic_errors() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"XXXX");
        let result = Bflyt::parse_file(&data);
        match result.unwrap_err() {
            FormatError::InvalidMagic { expected, .. } => assert_eq!(expected, "FLYT"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_header_size_larger_than_file_errors() {
        use crate::core::tchar_code32;
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&tchar_code32(b"FLYT").to_le_bytes());
        data[4..6].copy_from_slice(&0xFFFEu16.to_le_bytes());
        data[6..8].copy_from_slice(&0xFFFFu16.to_le_bytes());
        let result = Bflyt::parse_file(&data);
        match result.unwrap_err() {
            FormatError::InvalidHeaderSize {
                specified_size,
                actual_size,
            } => {
                assert!(specified_size > actual_size);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[cfg(test)]
mod bflan_file_tests {
    use crate::{
        bflan::file::Bflan,
        core::{FileReadWriteable, FormatError},
    };

    #[test]
    fn parse_empty_bytes_errors() {
        assert!(Bflan::parse_file(&[]).is_err());
    }

    #[test]
    fn parse_wrong_magic_errors() {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(b"XXXX");
        match Bflan::parse_file(&data).unwrap_err() {
            FormatError::InvalidMagic { expected, .. } => assert_eq!(expected, "FLAN"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_header_too_large_errors() {
        use crate::core::tchar_code32;
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&tchar_code32(b"FLAN").to_le_bytes());
        data[4..6].copy_from_slice(&0xFFFEu16.to_le_bytes());
        data[6..8].copy_from_slice(&0xFFFFu16.to_le_bytes());
        match Bflan::parse_file(&data).unwrap_err() {
            FormatError::InvalidHeaderSize { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[cfg(test)]
mod bflan_curve_tests {
    use crate::{
        bflan::curves::{Curve, HermiteKey, StepKey},
        core::{Cursor, Writer},
    };

    fn roundtrip_curve(curve: &Curve, curve_type: u8) -> Curve {
        let mut w = Writer::new();
        curve.serialize(&mut w);
        let frame_count = match curve {
            Curve::Constant(v) => v.len(),
            Curve::Step(v) => v.len(),
            Curve::Hermite(v) => v.len(),
        };
        let mut c = Cursor {
            data: &w.buffer,
            pos: 0,
            ..Default::default()
        };
        Curve::parse(&mut c, curve_type, frame_count).unwrap()
    }

    #[test]
    fn constant_curve_empty_roundtrip() {
        let curve = Curve::Constant(vec![]);
        let rt = roundtrip_curve(&curve, 0);
        assert!(matches!(rt, Curve::Constant(v) if v.is_empty()));
    }

    #[test]
    fn constant_curve_values_roundtrip() {
        let curve = Curve::Constant(vec![0.0, 0.5, 1.0]);
        let rt = roundtrip_curve(&curve, 0);
        if let Curve::Constant(vals) = rt {
            assert!((vals[0] - 0.0).abs() < 1e-6);
            assert!((vals[1] - 0.5).abs() < 1e-6);
            assert!((vals[2] - 1.0).abs() < 1e-6);
        } else {
            panic!("expected Constant");
        }
    }

    #[test]
    fn step_curve_roundtrip() {
        let curve = Curve::Step(vec![
            StepKey {
                frame: 0.0,
                value: 0,
            },
            StepKey {
                frame: 10.0,
                value: 255,
            },
        ]);
        let rt = roundtrip_curve(&curve, 1);
        if let Curve::Step(keys) = rt {
            assert_eq!(keys.len(), 2);
            assert!((keys[1].frame - 10.0).abs() < 1e-5);
            assert_eq!(keys[1].value, 255);
        } else {
            panic!("expected Step");
        }
    }

    #[test]
    fn hermite_curve_roundtrip() {
        let curve = Curve::Hermite(vec![
            HermiteKey {
                frame: 0.0,
                value: 0.0,
                slope: 1.0,
            },
            HermiteKey {
                frame: 5.0,
                value: 0.5,
                slope: 0.0,
            },
        ]);
        let rt = roundtrip_curve(&curve, 2);
        if let Curve::Hermite(keys) = rt {
            assert_eq!(keys.len(), 2);
            assert!((keys[1].value - 0.5).abs() < 1e-6);
        } else {
            panic!("expected Hermite");
        }
    }
}

#[cfg(test)]
mod bflan_enum_tests {
    use crate::{
        bflan::anim_info::{AnimInfoType, AnimType},
        core::tchar_code32,
    };

    #[test]
    fn anim_info_type_from_known_magic() {
        let magic = tchar_code32(b"FLPA");
        assert!(matches!(
            AnimInfoType::from(magic),
            AnimInfoType::PaneSrtAnim
        ));
    }

    #[test]
    fn anim_info_type_unknown_is_invalid() {
        assert!(matches!(
            AnimInfoType::from(0xDEAD_BEEF),
            AnimInfoType::Invalid
        ));
    }

    #[test]
    fn anim_info_type_all_known_tags_parse() {
        let cases: &[(&[u8; 4], AnimInfoType)] = &[
            (b"FLCC", AnimInfoType::PerCharacterTransformCurveAnim),
            (b"FLEU", AnimInfoType::ExtendedUserDataAnim),
            (b"FLCT", AnimInfoType::PerCharacterTransformAnim),
            (b"FLPA", AnimInfoType::PaneSrtAnim),
            (b"FLVC", AnimInfoType::VertexColorAnim),
            (b"FLVI", AnimInfoType::VisibilityAnim),
            (b"FLDS", AnimInfoType::DropShadowAnim),
            (b"FLMT", AnimInfoType::MaskTextureAnim),
            (b"FLPS", AnimInfoType::ProceduralShapeAnim),
            (b"FLWN", AnimInfoType::WindowAnim),
            (b"FSMA", AnimInfoType::StateMachineAnim),
            (b"FLAC", AnimInfoType::AlphaCompareAnim),
            (b"FLFS", AnimInfoType::FontShadowAnim),
            (b"FLIM", AnimInfoType::IndirectSrtAnim),
            (b"FLMC", AnimInfoType::MaterialColorAnim),
            (b"FLTS", AnimInfoType::TextureSrtAnim),
            (b"FLTP", AnimInfoType::TexturePatternAnim),
            (b"FTBR", AnimInfoType::BrickRepeatAnim),
            (b"FVGA", AnimInfoType::VectorGraphicsAnim),
        ];

        for (tag, expected) in cases {
            let got = AnimInfoType::from(tchar_code32(*tag));
            assert_eq!(
                std::mem::discriminant(&got),
                std::mem::discriminant(expected),
                "tag {:?} did not map to expected variant",
                tag
            );
        }
    }

    #[test]
    fn anim_type_discriminants_are_stable() {
        assert_eq!(AnimType::Pane as u8, 0);
        assert_eq!(AnimType::Material as u8, 1);
        assert_eq!(AnimType::User as u8, 2);
        assert_eq!(AnimType::PaneExt as u8, 3);
        assert_eq!(AnimType::StateMachine as u8, 4);
    }
}

#[cfg(test)]
mod error_display_tests {
    use crate::core::FormatError;

    #[test]
    fn invalid_magic_display() {
        let e = FormatError::InvalidMagic {
            expected: "FLYT",
            found: 0xDEAD_BEEF,
            offset: 0,
        };
        let s = e.to_string();
        assert!(s.contains("FLYT"));
    }

    #[test]
    fn unexpected_eof_display() {
        let e = FormatError::UnexpectedEof {
            offset: 4,
            requested_bytes: 8,
        };
        let s = e.to_string();
        assert!(s.contains("EOF"));
    }

    #[test]
    fn malformed_section_display() {
        let e = FormatError::MalformedSection {
            section_type: "txl1".to_string(),
            offset: 0x10,
            reason: "bad offset".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("txl1"));
        assert!(s.contains("bad offset"));
    }

    #[test]
    fn section_count_mismatch_display() {
        let inner = FormatError::UnexpectedEof {
            offset: 0,
            requested_bytes: 4,
        };
        let e = FormatError::SectionCountMismatch {
            expected: 5,
            actual: 2,
            source: Box::new(inner),
        };
        let s = e.to_string();
        assert!(s.contains('5'));
        assert!(s.contains('2'));
    }

    #[test]
    fn invalid_header_size_display() {
        let e = FormatError::InvalidHeaderSize {
            specified_size: 1000,
            actual_size: 10,
        };
        let s = e.to_string();
        assert!(s.contains("1000") || s.contains("0x3E8"));
    }
}
