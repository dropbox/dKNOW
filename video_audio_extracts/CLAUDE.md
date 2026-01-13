# CLAUDE.md - <Title of this Project>

<One line description of this project>

**Author:** Andrew Yates
**Copyright:** 2026 Dropbox, Inc. | **License:** Apache 2.0
**Repo:** https://github.com/ayates_dbx/<the repo>
**Location:** `~/<the path>/`
**<if porting> Baseline Local:** `~/<filepath>` (never edit this) <versioning>
**<if porting> Baseline Remote:** `<https://github.com/remote repo>`

> **Project initialization:**
> 1. Run `./ai_template_scripts/init_from_template.sh` (removes template files, sets up labels)
> 2. Replace all `<placeholders>` in CLAUDE.md with project-specific values
> 3. Delete lines marked `<if porting>` if not applicable
> 4. Run `./ai_template_scripts/init_from_template.sh --verify` to check all placeholders replaced
> 5. Replace `README.md` with project content
> 6. Write initial design document and roadmap

**Roadmap workflow** (run at init; re-run when all items complete, directed by MANAGER, major scope change, or consolidating ad-hoc requests):
1. Check existing issues: `gh issue list --state all`
2. Write `ROADMAP.md` documenting goals and tasks. Review with 2+ passes: skeptical, rigorous, ambitious. Git commit.
3. Create issues directly: `gh issue create --title "Task" --body "Description" --label "P1,task"`
4. `ROADMAP.md` stays in repo as record of original roadmap

**Ideas:** `IDEAS.md` captures future considerations and backlog items. Not actionable - don't convert to issues unless directed.

---

<!-- TEMPLATE: Header + init instructions are universal. "Project-Specific Configuration" section below is for customization. -->


## Project-Specific Configuration

**Primary languages:** <Your primary languages: Rust, C++, and Python>

<CUSTOMIZE THESE RULES AT INITIALIZATION FOR THE TARGET PROJECT AS APPROPRIATE. BE CONCISE AND STRATEGIC IN YOUR DIRECTION>

- <only if porting> **WHEN STUCK, CHECK BASELINE SOURCE FIRST**

- <only if porting from Python> **NO PYTHON IN RUNTIME**: Final binary must run without Python. Python allowed only for dev scripts (ONNX export, validation, corpus generation). If blocked, use C++.

- <if appropriate> **C++ IS ALLOWED AND ENCOURAGED**: C++ dependencies are acceptable and encouraged when appropriate. Prefer Rust for new logic you write, but use C++ libraries when they provide tested, production-quality implementations.

- <only if porting> **Study Python Source Code First**: When porting Python functionality, examine the baseline source code in "Baseline Local" before implementing. Do NOT write heuristics to fit test outputs. Port the actual baseline algorithms first. Test outputs show WHAT to produce, Baseline source shows HOW.

**Goals:**
1. As fast as absolutely possible.
2. Perfect code quality.
<customize goals for the project>

<Additional Project Customizations and Goals>
