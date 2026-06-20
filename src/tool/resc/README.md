# YAML resource config

## What's the benefit of the YAML format compared to the XML manifest used by the game?

There are many:

- **Pattern matching**: You can define a pattern (glob or regex) to match a list of files
  and the corresponding XML entries will be generated for each of the matched file.
  No need to edit the manifest manually when adding new resources.
- **Checked**: If the manifest references a file that doesn't exist, you will get a compile-time error,
  instead of a runtime error dialog.

## What's the difference of this tool compared to the original ResourceGen tool?

This tools comes with a bit more customization such as namespace and header paths.
It also does not generate functions that are unlikely to be useful (such as getting a reference
to a static global variable)

It also generates `enum class` instead of `enum`s.

## Config Format

### Input/Output and Codegen Options
The input and output options are also baked into the same config:
```yaml
# input and output options
paths:
  input-directory: path/to/unpacked/pak
    # ^ the pak directory to look for resources
  output-xml: path/to/unpacked/pak/properties/resources.xml
    # ^ the place to write the output XML manifest
  output-cpp: path/to/src/Resources.cpp
    # ^ the place to write the output CPP file
  # output-h: path/to/src/Resources.h
    # ^ the place to write the output header file. By default it will be next to the CPP
    #   file, with .cpp changed to .h
  excludes:
    # you can exclude files and directories here
    - glob:compiled/**/*
    - regex:^properties

# codegen (cpp and header file) options
codegen:
  # include-prefix-sexy: SexyAppFramework
    # ^ prefix for including from SexyAppFramework. Default is "SexyAppFramework"
    #   for example the include statement will be #include <SexyAppFramework/ResourceManager.h>
  include-prefix: LawnApp
    # ^ prefix for including the generated header.
    #   by default, it will be #include "NAME.h" where <name> is the file stem of the CPP file
    #   if you specify something (LawnApp in this example), the include will be #include <LawnApp/NAME.h>
  # namespace: Sexy
    # ^ namespace for generating your resources, default is Sexy


resource-groups:
  # ... definition of resources, see below
```

### Resources

The `resource-groups` should be an array of groups, each with `name` and `contents`
```yaml
resource-groups:
  - name: LoaderBar
    # ^ name of the group (corresponds to id in the <Resources> tag)
    contents:
      # you can think of each "content" to be a section with <SetDefaults> on top of it
      - type: Image
          # ^ type can be Image, Font or Sound
        defaults: { path: images, id-prefix: IMAGE_ }
          # ^ this is the same as <SetDefaults> in the XML manifest
        attrs:
          # default attributes to be applied (can be overriden at per-image level)
          a8r8g8b8: true
        items:
          - path: titlescreen.jpg
            # ^ there are several ways to define an item
            #   the easiest is using just path, the ID will be inferred
          - pattern: glob:LoadBar_*.png
            # ^ or you can use a pattern (glob:... or regex:...), which will define one item
            #   per matched file. Note that each file that gets added will be "consumed"
            #   and will no longer be matched by other patterns
            #   The ID is also inferred in this case and you cannot override the ID for a pattern
  - name: LoadingImages
    contents:
      - type: Image
        defaults: { path: images, id-prefix: IMAGE_ }
        items:
          - id: ZOMBIE_NOTE_SMALL
            path: ZombieNoteSmall.png
            # ^ if the ID you want to use is different from the inferred ID, you can override it
          - path: FlagMeter.png
            attrs: 
              # you can specify attributes at a per-image level
              rows: 2
              minsubdivide: true
          - { id: COIN_GOLD_DOLLAR, path: Coin_gold_dollar, external: true }
            # ^ you can use 'external' to add the ID and PATH as-is to the XMl manifest
            #   in this case the existence of this file won't be checked
            #   (this was useful to re-create the XML manifest from the original game
            #     when the file casing mismatches)
```

### Special Image Attributes
When using `alphaimage` and `alphagrid`, you can put `[name]` as a placeholder of the file stem
when using pattern macthing:
```yaml
resource-groups:
  - name: MyGroup
    contents:
      - type: Image
        defaults: { path: images, id-prefix: IMAGE_ }
        items:
          - pattern: regex:My_Image_[a-zA-Z0-9_]+.png
            # ^ for example this can match "My_Image_hello.png"
            attrs:
              alphaimage: '[name].alpha.png'
                # ^ and this will be My_Image_hello.alpha.png
```

### Sys Font 

When loading a sys font, set `sys: true` and `external: true`

```yaml
resource-groups:
  - name: MyGroup
    contents:
      - type: Font
        defaults: { path: data, id-prefix: FONT_ }
        items:
          - id: MY_SYS_FONT
            path: mysysfontname # you don't need the !sys prefix or the .ttf suffix
            sys: true
            external: true
            attrs:
              bold: true
                # ^ you can add the sys font attributes here
```
