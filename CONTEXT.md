# rokf

rokf helps people and agents create, inspect, and maintain Open Knowledge Format knowledge bundles.

## Language

**Open Knowledge Format (OKF)**:
An open, human- and agent-friendly format for representing knowledge as a portable bundle of markdown documents with structured frontmatter.
_Avoid_: OKF schema registry, knowledge database

**Knowledge**:
The metadata, context, and curated insight that surrounds data and systems.
_Avoid_: Raw data, system of record

**Knowledge Bundle**:
A self-contained, hierarchical collection of knowledge documents. The unit of distribution for OKF.
_Avoid_: Corpus, repository, package

**Bundle Root**:
The top directory of a Knowledge Bundle; absolute bundle-relative links are resolved from this directory.
_Avoid_: Project root, repository root

**Bundle Discovery**:
Determining the Bundle Root for a Verification or Authoring workflow.
_Avoid_: Root detection, project discovery

**Bundle Hierarchy**:
The directory tree inside a Knowledge Bundle that organizes concepts into groups.
_Avoid_: Folder hierarchy, taxonomy

**Traversal**:
Walking a Knowledge Bundle's Bundle Hierarchy to discover OKF Documents and relationships.
_Avoid_: Filesystem walk, crawl

**Concept**:
A single unit of knowledge within a Knowledge Bundle. A Concept may describe a tangible asset, an abstract idea, or anything in between.
_Avoid_: Page, entry, node

**OKF Document**:
Any markdown file with defined meaning inside a Knowledge Bundle, including Concept Documents, Index Files, and Log Files.
_Avoid_: OKF file

**Reserved File**:
An OKF Document whose filename has defined meaning and is not a Concept Document.
_Avoid_: Special file, reserved document

**Concept Document**:
The markdown document that represents one Concept in a Knowledge Bundle.
_Avoid_: OKF file, page file

**Concept ID**:
The bundle-relative path of a Concept Document with its `.md` suffix removed.
_Avoid_: Slug, document ID, filename

**Frontmatter**:
The structured metadata block at the top of a Concept Document.
_Avoid_: Header, metadata header

**Producer-defined Field**:
A frontmatter field outside the OKF-required and recommended fields, added by a Producer for its own use.
_Avoid_: Custom field, extension field

**Body**:
The markdown content that follows frontmatter in a Concept Document.
_Avoid_: Content, prose section

**Concept Type**:
The required frontmatter value that identifies what kind of Concept a Concept Document represents.
_Avoid_: Type, class, category, schema type

**Title**:
An optional human-readable display name for a Concept.
_Avoid_: Name, label

**Description**:
An optional one-sentence summary of a Concept.
_Avoid_: Summary, abstract

**Underlying Asset**:
The real-world or digital thing a Concept describes when the Concept is bound to something outside the Knowledge Bundle.
_Avoid_: Resource, source object

**Resource**:
An optional canonical URI for the Underlying Asset described by a Concept.
_Avoid_: Source, target, asset link

**Tag**:
A short frontmatter value used for cross-cutting categorization of Concepts.
_Avoid_: Label, keyword

**Timestamp**:
An optional frontmatter value describing the last meaningful change to a Concept.
_Avoid_: Modified date, updated at

**Relationship**:
A connection from one Concept to another expressed by a Link and clarified by surrounding prose.
_Avoid_: Typed edge, dependency

**Link**:
A markdown link from one Concept to another that expresses a relationship through its surrounding prose.
_Avoid_: Edge, reference link

**Broken Link**:
A Link whose target does not exist in the Knowledge Bundle at Verification time.
_Avoid_: Invalid link, failed reference

**Citation**:
A link from a Concept to an external source that supports a claim in the Body.
_Avoid_: Source, reference

**Index File**:
A reserved file that lists the contents of a directory in a Knowledge Bundle for progressive disclosure.
_Avoid_: Table of contents, directory page

**Root Index File**:
The Index File at the Bundle Root.
_Avoid_: Manifest, bundle manifest

**Progressive Disclosure**:
Presenting a Knowledge Bundle in layers so a Producer or Consumer can discover available knowledge before opening individual Concept Documents.
_Avoid_: Full-text index, exhaustive summary

**Index Maintenance**:
Maintaining Index Files so Progressive Disclosure reflects the current Bundle Hierarchy and Concept Documents.
_Avoid_: Indexing, search indexing

**Version Declaration**:
The optional `okf_version` value that states which OKF version a Knowledge Bundle targets.
_Avoid_: Manifest version, schema version

**Log File**:
A reserved file that records the history of changes for a scope within a Knowledge Bundle.
_Avoid_: Changelog, history file

**Producer**:
A person, agent, or system that creates or updates OKF content.
_Avoid_: Writer, authoring client

**Consumer**:
A person, agent, or system that reads or traverses OKF content.
_Avoid_: Reader, client

**Agent-friendly**:
A quality of OKF tooling that makes workflows concise, predictable, structured, and safe for AI agents to use in automation.
_Avoid_: Token-efficient, machine-readable

**Structured Output**:
rokf output designed to be parsed reliably by tools or agents.
_Avoid_: Machine output, JSON output

**Authoring**:
The creation or revision of OKF Documents.
_Avoid_: Writing, generation

**Direct Authoring**:
Authoring OKF Documents without using rokf authoring commands, followed by Verification to close the loop.
_Avoid_: Manual writing, bypassing rokf

**Assisted Authoring**:
Authoring OKF Documents through rokf commands that produce predictable, structured output.
_Avoid_: Command authoring, deterministic authoring

**Document Template**:
A reusable starting shape for authoring an OKF Document.
_Avoid_: Scaffold, boilerplate

**Round-trip**:
Reading an OKF Document and writing it back without losing unknown or producer-defined content.
_Avoid_: Rewrite, regeneration

**Maintenance**:
The ongoing care of a Knowledge Bundle as concepts are added, moved, updated, or checked.
_Avoid_: Upkeep, management

**Verification**:
The umbrella activity of checking OKF content and reporting whether it is acceptable for a given use.
_Avoid_: Checking, auditing

**Self Verification**:
A workflow where a Producer verifies its own OKF output before handing it off.
_Avoid_: Authoring loop, self-check

**Verification Scope**:
The OKF content covered by a Verification workflow.
_Avoid_: Target, selection

**Check**:
A Verification workflow that reports Findings without applying Fixes or Formatting.
_Avoid_: Verify command, lint command

**Document Verification**:
Verification of one OKF Document in isolation.
_Avoid_: File verification, single-file validation

**Bundle Verification**:
Verification that traverses a Knowledge Bundle and checks relationships across the Bundle Hierarchy.
_Avoid_: Folder verification, recursive validation

**Rule**:
A named expectation that can produce Findings during Verification.
_Avoid_: Check, lint

**Rule Code**:
A stable identifier for a Rule, used in Findings, configuration, and documentation.
_Avoid_: Error code, diagnostic code

**Configuration**:
The declared choices that shape rokf workflows for a Knowledge Bundle.
_Avoid_: Settings, options

**Rule Set**:
A group of Rules used for a Verification workflow.
_Avoid_: Profile, config preset

**Conformance Rule**:
A Rule derived from a required OKF specification constraint.
_Avoid_: Validation rule, hard rule

**Quality Rule**:
A Rule derived from optional guidance, maintainability expectations, or agent-friendly conventions.
_Avoid_: Lint rule, soft rule

**Finding**:
A reported result from Verification.
_Avoid_: Message, issue, diagnostic

**Severity**:
The level assigned to a Finding that communicates its impact on OKF conformance, quality, or maintainability.
_Avoid_: Priority, importance

**Suppression**:
An explicit decision not to report a Rule's Finding in a given scope.
_Avoid_: Ignore, exclusion

**Exclusion**:
An explicit decision to leave OKF content outside a Verification Scope.
_Avoid_: Suppression, skip

**Error**:
A Finding that means required OKF conformance is broken.
_Avoid_: Failure, violation

**Warning**:
A Finding that means guidance or quality expectations are not met, but the content can still be consumed.
_Avoid_: Alert, notice

**Suggestion**:
A Finding that recommends an improvement without implying the content is currently wrong.
_Avoid_: Hint, tip

**Validation**:
Checking OKF content against hard conformance rules from the OKF specification.
_Avoid_: Verification, linting

**Linting**:
Checking OKF content against soft guidance, quality expectations, maintainability concerns, and agent-friendly conventions.
_Avoid_: Validation, formatting

**Format Check**:
Reporting whether OKF Documents already match normalized presentation without changing them.
_Avoid_: Formatting validation, dry-run formatting

**Formatting**:
Normalizing the presentation of OKF Documents without changing their meaning.
_Avoid_: Linting, rewriting

**Fix**:
An automated change that resolves a Finding without changing the intended meaning of OKF content.
_Avoid_: Repair, correction

**Fixable Finding**:
A Finding that rokf can resolve automatically with a Fix.
_Avoid_: Auto-fixable issue, repairable finding

**Conformant Bundle**:
A Knowledge Bundle that satisfies the required OKF rules for concept frontmatter and reserved files.
_Avoid_: Valid bundle, compliant bundle

**Healthy Bundle**:
A Knowledge Bundle that is conformant and has no lint findings above the chosen threshold.
_Avoid_: Valid bundle, compliant bundle, clean bundle

**Failure Threshold**:
The minimum Finding severity that causes Verification to fail.
_Avoid_: Strictness, fail level
