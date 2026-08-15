# nnbfl

nnbfl (Nintendo Binary File Layout) is a serializer & deserializer for .bflan & .bflyt files.
Tested with:

- Tomodachi Life Living The Dream
- Animal Crossing New Horizons
- Mario Kart 8
- Super Smash Bros. Ultimate
- The Legend of Zelda: Tears of the Kingdom

## Supported formats

- .bflan
- .bflyt
- .bfcpx
- .bfttf / .bfotf

# Usage

## Extracting files

```sh
# Single file conversion
nnbfl bflyt extract MiiTouch_RelationRanking_00.bflyt layout_dump.json
nnbfl bflan extract MiiTouch_RelationRanking_00_TypeArrow.bflan anim_dump.json

# Batch process an entire folder
nnbfl bflyt extract /path/to/bflyt_dir/ /path/to/output_json_dir/
nnbfl bflan extract /path/to/bflan_dir/ /path/to/output_json_dir/
```

## Packing files

```sh
# Single file conversion
nnbfl bflyt pack layout_dump.json Modified_Layout_00.bflyt
nnbfl bflan pack anim_dump.json Modified_Anim_00.bflan

# Batch process an entire folder
nnbfl bflyt pack /path/to/bflyt_dir/ /path/to/output_json_dir/
nnbfl bflan pack /path/to/bflan_dir/ /path/to/output_json_dir/
```

## Validating the accuracy of the output

```sh
# Test a single layout or an entire folder
nnbfl bflyt test MiiTouch_RelationRanking_00.bflyt
nnbfl bflyt test /path/to/bflyt_dir/

# Test a single animation or an entire folder
nnbfl bflan test MiiTouch_RelationRanking_00_TypeArrow.bflan
nnbfl bflan test /path/to/bflan_dir/
```

# Credits

- Watertoon for their hexpat on .bflan & .bflyt files.
- KillzXGaming ([LayoutLibrary](https://github.com/KillzXGaming/LayoutLibrary)) for reference implementation about `MaterialDetailedCombiner`.

# License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
