# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---
## [unreleased]

### Bug Fixes

- **(ci)** declare pnpm version via packageManager field - ([73e4a68](https://github.com/zlx2019/git-pincer-desktop/commit/73e4a6880010e5cfa19a70eb8f257da7f716cb1a)) - Zero
- **(merge)** guard word-level emphasis against overlong lines - ([05ed232](https://github.com/zlx2019/git-pincer-desktop/commit/05ed232f7d5e1690178bd9976cab1a0432e08603)) - Zero
- **(merge)** mirror agree-chunk word emphasis to the right pane - ([5fbc5f1](https://github.com/zlx2019/git-pincer-desktop/commit/5fbc5f1ae66e31b1076e72f03e1787e3236b2ef0)) - Zero
- **(merge)** drop CSS containment that ghosts seam bands under WKWebView - ([6d091ae](https://github.com/zlx2019/git-pincer-desktop/commit/6d091aec97cf8bfcf0935b6f383d07707a5d3a16)) - Zero
- **(merge-view)** live ruler geometry, symmetric batch apply and culled seams - ([f4bfbde](https://github.com/zlx2019/git-pincer-desktop/commit/f4bfbde74161ea112d1116782396c210ce250cc5)) - Zero
- **(repo)** resolve rebase onto label and scope accept_side stage lookup - ([837d854](https://github.com/zlx2019/git-pincer-desktop/commit/837d8544c04b8bce4603a35484526724729cea57)) - Zero
- **(ui)** use amber for the busy status and set the app title - ([40cd620](https://github.com/zlx2019/git-pincer-desktop/commit/40cd62015aa7b3f6e1b5c44a40ea2794065009d5)) - Zero
- **(window)** restore each form's last position instead of recentering - ([c57241f](https://github.com/zlx2019/git-pincer-desktop/commit/c57241f9881bf2bef92b0ac5e49df3b530884adb)) - Zero

### Documentation

- add README, contributing guide and design documents - ([a5df7ed](https://github.com/zlx2019/git-pincer-desktop/commit/a5df7ed697d18bc31ba576db27dcd54bb1e6e478)) - Zero
- add CLAUDE.md with build commands and architecture guide - ([8447322](https://github.com/zlx2019/git-pincer-desktop/commit/844732226a16ec540f032d6ea231c847b6d10266)) - Zero
- align CLAUDE.md with the implementation and add known-issues backlog - ([ef5540e](https://github.com/zlx2019/git-pincer-desktop/commit/ef5540e8bc1b4e53437274169dd44102e2400da4)) - Zero
- record settled decisions and clear the issue backlog - ([9b0f362](https://github.com/zlx2019/git-pincer-desktop/commit/9b0f362eac74ffde5f2ad045340c23908a5d0c5e)) - Zero
- record performance and window-shell decisions - ([9497b00](https://github.com/zlx2019/git-pincer-desktop/commit/9497b00a7c8d29a75949a3bc32885910963813b4)) - Zero
- add open-source maturity checklist - ([65f6364](https://github.com/zlx2019/git-pincer-desktop/commit/65f6364da1d7e6942b5f895b3b3149a067f873c5)) - Zero
- record the settings system decisions - ([5604afb](https://github.com/zlx2019/git-pincer-desktop/commit/5604afb28f257e0f25ae986a0246dcc5bb80ae0b)) - Zero
- record theme and language decisions - ([c4e034a](https://github.com/zlx2019/git-pincer-desktop/commit/c4e034a4c500a48a2a33697c594e6f223f2ed3b1)) - Zero

### Features

- **(engine)** add conservative three-way merge chunking engine - ([50530c9](https://github.com/zlx2019/git-pincer-desktop/commit/50530c9a10de50515bee96238f85bb477635e23b)) - Zero
- **(merge)** add IDEA-style shared horizontal scrollbar under the panes - ([8ad6ef4](https://github.com/zlx2019/git-pincer-desktop/commit/8ad6ef4bd734de74b6c12b2f079439b80cf91051)) - Zero
- **(repo)** add branch switching, recent-list removal and commit source refs - ([854f84c](https://github.com/zlx2019/git-pincer-desktop/commit/854f84c9519e1c261718612e5529ead5a85554dd)) - Zero
- **(settings)** typed persistent user settings - ([7a808a8](https://github.com/zlx2019/git-pincer-desktop/commit/7a808a8b9e873f7f3db55fd7d0599aa1d9608af3)) - Zero
- **(settings)** theme and language options with native shell sync - ([fcf7018](https://github.com/zlx2019/git-pincer-desktop/commit/fcf7018d9760d54467ba34561edcb7699186c36e)) - Zero
- **(settings)** open settings from the macOS app menu and Cmd+, - ([484631b](https://github.com/zlx2019/git-pincer-desktop/commit/484631b6b2227976d4e0e2b43c9533c8964a43a2)) - Zero
- **(settings)** bundle Maple Mono as a second embedded editor font - ([c7d9a69](https://github.com/zlx2019/git-pincer-desktop/commit/c7d9a69bb80cc8bf2c6dcda43491926dcf37e54c)) - Zero
- **(shell)** add stateless git plumbing and tauri command layer - ([f3bd3c9](https://github.com/zlx2019/git-pincer-desktop/commit/f3bd3c9859e0e060189b25e1d33d676b5912cbc9)) - Zero
- **(shell)** keep running in the system tray when the window closes - ([fea51e4](https://github.com/zlx2019/git-pincer-desktop/commit/fea51e43c04fab16ca902da0dd20941d313f1f92)) - Zero
- **(theme)** embed JetBrains Mono Regular - ([ee81d12](https://github.com/zlx2019/git-pincer-desktop/commit/ee81d12251de2f6591d4bb1177cbe734c725de12)) - Zero
- **(ui)** add command palette, conflicts list and three-pane merge editor - ([28d4fe5](https://github.com/zlx2019/git-pincer-desktop/commit/28d4fe5ba43f1852e8f9ad744718f9a4c92a38c8)) - Zero
- **(ui)** polish merge view and unify in-app feedback - ([1fa156f](https://github.com/zlx2019/git-pincer-desktop/commit/1fa156f26b45bd417c489a2346c0fe5b139fdad7)) - Zero
- **(ui)** park an in-progress operation and resume it from the menu - ([52fb779](https://github.com/zlx2019/git-pincer-desktop/commit/52fb77940d00400145830264e2eff8998dbdf4e5)) - Zero
- **(ui)** settings dialog with editor font and behavior options - ([49a5454](https://github.com/zlx2019/git-pincer-desktop/commit/49a545417d3fed114887ded051f23061f61585cc)) - Zero
- **(ui)** light theme and bilingual copy driven by settings - ([a828d0a](https://github.com/zlx2019/git-pincer-desktop/commit/a828d0a464c94e87e7bc725756e120b11264509f)) - Zero
- **(window)** persist per-form window size across launches - ([9b07a5c](https://github.com/zlx2019/git-pincer-desktop/commit/9b07a5c12cef3e64a7face8c0d2a194a9b53857a)) - Zero

### Miscellaneous Chores

- scaffold Tauri 2 + SvelteKit application shell - ([b148dab](https://github.com/zlx2019/git-pincer-desktop/commit/b148dab8ce268fbaeaa10bee4f12dbe88cec3fae)) - Zero
- align lint, audit and pre-commit tooling with rust-template - ([3749cf9](https://github.com/zlx2019/git-pincer-desktop/commit/3749cf96e118136b610a110b99a921ffc9af9f84)) - Zero

### Other

- **(release)** abort on panic for a leaner binary - ([d6bbc8f](https://github.com/zlx2019/git-pincer-desktop/commit/d6bbc8fbd9c7dac895b09913213a4071dd5cd896)) - Zero
- **(tauri)** migrate config from tauri.conf.json to Tauri.toml - ([d6a9913](https://github.com/zlx2019/git-pincer-desktop/commit/d6a9913ede5cf0f52d14ec57ea033139b2a83f81)) - Zero
- add GitHub Actions CI, release pipeline and dependabot - ([71159f2](https://github.com/zlx2019/git-pincer-desktop/commit/71159f2c97807631818f7c712482a56a2a0731df)) - Zero

### Performance

- **(build)** optimize cargo profiles and vite build targets - ([ded59eb](https://github.com/zlx2019/git-pincer-desktop/commit/ded59ebe601af9ab01771af35f6aa6feffa15869)) - Zero
- **(editor)** clip decorations to viewport and dedupe seam geometry - ([c4fc947](https://github.com/zlx2019/git-pincer-desktop/commit/c4fc947edc2d6e199cd87058df5c7a87a9af441c)) - Zero
- **(shell)** switch window forms in one IPC and remove startup flash - ([72d7e6b](https://github.com/zlx2019/git-pincer-desktop/commit/72d7e6b223dcb95ea0938e6cbf00fc487134d945)) - Zero
- **(ui)** batch git output per frame and rate-limit focus reprobes - ([f0edabf](https://github.com/zlx2019/git-pincer-desktop/commit/f0edabfa7bf998b88d222babe1a6ec6e8968c45a)) - Zero

### Tests

- **(ui)** introduce vitest for the extracted merge logic - ([f189c69](https://github.com/zlx2019/git-pincer-desktop/commit/f189c697252f97cc4eca66f9e00e124aff111fef)) - Zero

<!-- generated by git-cliff -->
