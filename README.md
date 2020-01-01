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

It can load a package file and save it later, it will calculate sha256 automatically as you save, and my favorite: URL analysis

## URL analysis
The button "From URL" will ask for a URL pointing to a file and will try to infer as much data from there. Data obtained right now

- Name: Either from the file or from earlier in the URL (this last one only follows Github relases name scheme), won't replace the current one if is not empty
- Version: From file name
- Source: (Well, of course)
- Summary: If it came from a Github repo will load the main page and get the summary (buggy right now though)

Also it's made so that updating an existing package is a matter of using "From URL" and saving