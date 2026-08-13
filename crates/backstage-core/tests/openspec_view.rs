use backstage_core::{
    OpenSpecOverviewKind, OpenSpecProgress, OpenSpecSource, build_openspec_view,
    parse_openspec_tasks,
};

#[test]
fn extracts_canonical_proposal_and_design_sections_without_fenced_headings() {
    let sources = vec![
        OpenSpecSource {
            relative_path: "openspec/changes/better-viewer/design.md".to_owned(),
            markdown: "## Goals / Non-Goals\n\n**Goals:** Make work clear.\n\n## Decisions\n\n### Parse locally\n\nFacts stay deterministic.\n\n## Risks / Trade-offs\n\n- Missing sections are omitted.\n".to_owned(),
        },
        OpenSpecSource {
            relative_path: "openspec/changes/better-viewer/proposal.md".to_owned(),
            markdown: "# Better viewer\n\n## Why\n\nUsers need context.\n\n```md\n## What Changes\nIgnored example\n```\n\n## What Changes\n\n- Add overview\n".to_owned(),
        },
    ];

    let view = build_openspec_view(&sources, &parse_openspec_tasks("# Tasks\n"));

    assert_eq!(
        view.overview
            .iter()
            .map(|section| section.kind)
            .collect::<Vec<_>>(),
        vec![
            OpenSpecOverviewKind::Why,
            OpenSpecOverviewKind::WhatChanges,
            OpenSpecOverviewKind::GoalsAndNonGoals,
            OpenSpecOverviewKind::Decisions,
            OpenSpecOverviewKind::RisksAndTradeOffs,
        ]
    );
    assert!(view.overview[0].markdown.starts_with("Users need context."));
    assert!(
        view.overview[0]
            .markdown
            .contains("## What Changes\nIgnored example")
    );
    assert_eq!(view.overview[1].markdown, "- Add overview");
    assert!(view.overview[3].markdown.contains("### Parse locally"));
}

#[test]
fn recognizes_canonical_heading_aliases_and_omits_empty_sections() {
    let sources = vec![
        OpenSpecSource {
            relative_path: "proposal.md".to_owned(),
            markdown: "## WHY\n\nA reason.\n\n## What changes?\n\n\n".to_owned(),
        },
        OpenSpecSource {
            relative_path: "design.md".to_owned(),
            markdown: "## Goals/Non Goals\n\nA boundary.\n\n## Risks and Tradeoffs\n\nA risk.\n"
                .to_owned(),
        },
    ];

    let view = build_openspec_view(&sources, &parse_openspec_tasks("# Tasks\n"));

    assert_eq!(view.overview.len(), 3);
    assert_eq!(view.overview[0].kind, OpenSpecOverviewKind::Why);
    assert_eq!(
        view.overview[1].kind,
        OpenSpecOverviewKind::GoalsAndNonGoals
    );
    assert_eq!(
        view.overview[2].kind,
        OpenSpecOverviewKind::RisksAndTradeOffs
    );
}

#[test]
fn groups_all_task_facts_under_their_nearest_source_heading() {
    let tasks = "# Tasks\n\n- [x] Before groups\n\n## 1. Foundation\n\n- [x] Parse sections\n- [ ] Test aliases\n\n### 1.1 Integration\n\n- [x] Return the view\n\n```md\n## Fake group\n- [ ] Fake task\n```\n\n## 2. Interface\n\n- [ ] Build overview\n";
    let progress = parse_openspec_tasks(tasks);
    let sources = vec![OpenSpecSource {
        relative_path: "openspec/changes/better-viewer/tasks.md".to_owned(),
        markdown: tasks.to_owned(),
    }];

    let view = build_openspec_view(&sources, &progress);

    let OpenSpecProgress::Available(progress) = progress else {
        panic!("task progress");
    };
    assert_eq!(view.task_groups.len(), 4);
    assert_eq!(view.task_groups[0].title, "Other tasks");
    assert_eq!(view.task_groups[1].title, "1. Foundation");
    assert_eq!(view.task_groups[2].title, "1.1 Integration");
    assert_eq!(view.task_groups[3].title, "2. Interface");
    assert_eq!(
        view.task_groups
            .iter()
            .flat_map(|group| &group.tasks)
            .collect::<Vec<_>>(),
        progress.tasks.iter().collect::<Vec<_>>()
    );
    assert!(view.task_groups[1].tasks[0].completed);
    assert!(!view.task_groups[1].tasks[1].completed);
}

#[test]
fn longer_outer_fences_keep_embedded_headings_and_tasks_inert() {
    let proposal = "````md\n## Why\nFake reason\n```\n## What Changes\n- [ ] Fake change\n````\n\n## Why\n\nReal reason.\n";
    let tasks = "# Tasks\n\n````md\n## Fake group\n```\n- [ ] Fake task\n````\n\n## Real group\n\n- [x] Real task\n";
    let sources = vec![
        OpenSpecSource {
            relative_path: "proposal.md".to_owned(),
            markdown: proposal.to_owned(),
        },
        OpenSpecSource {
            relative_path: "tasks.md".to_owned(),
            markdown: tasks.to_owned(),
        },
    ];

    let view = build_openspec_view(&sources, &parse_openspec_tasks(tasks));

    assert_eq!(view.overview.len(), 1);
    assert_eq!(view.overview[0].kind, OpenSpecOverviewKind::Why);
    assert_eq!(view.overview[0].markdown, "Real reason.");
    assert_eq!(view.task_groups.len(), 1);
    assert_eq!(view.task_groups[0].title, "Real group");
    assert_eq!(view.task_groups[0].tasks.len(), 1);
    assert_eq!(view.task_groups[0].tasks[0].text, "Real task");
}

#[test]
fn unavailable_task_progress_produces_no_invented_groups() {
    let source = OpenSpecSource {
        relative_path: "tasks.md".to_owned(),
        markdown: "# Tasks\n\nTasks live elsewhere.\n".to_owned(),
    };

    let view = build_openspec_view(&[source], &parse_openspec_tasks("# Tasks\n"));

    assert!(view.task_groups.is_empty());
}
