#packmak
Short for Package Make, an utility designed to make making Solus packages much, mcuh easier (and faster)


It serves as a GUI for editing some data in a package:
- Name
- Version
- Component
- Source
- License
- Build System
- Summary
- Description

It will save any new package on a folder next to the executable with the same name as the package and the data inside a package.yml, as is standard on Solus.

It can load a package file and save it later, it will calculate sha256 automatically as you save, and my favorite: URL analysis.

## URL analysis
The button "From URL" will ask for a URL pointing to a file and will try to infer as much data from there. Data obtained right now

- Name: Either from the file or from earlier in the URL (this last one only follows Github relases name scheme), won't replace the current one if is not empty
- Version: From file name
- Source: (Well, of course)
- Summary: If it came from a Github repo will load the main page and get the summary (buggy right now though)
- Build system: Only for zip files, and only detects meson
- License: Only for zip files, and only detects GPL-3

Also it's made so that updating an existing package is a matter of using "From URL" and saving.

## Buginess
This program is VERY buggy (just look at the miriads of unwrap all around), that said, this won't break anything, this program will panic (a lot probably), but is expected to never crash, so undefined behaviour shouldn't be possible (thanks Rust).

In other words, this program will shut down itself a lot, but I have certain faith that it won't ruin anything (not battle tested, so a backup is always desirable).

Also the GUI can be improved, I know (specially for )