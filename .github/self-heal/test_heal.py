import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('heal', Path(__file__).with_name('heal.py'))
heal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(heal)


def event(conclusion='failure'):
    return dict(repository=dict(owner=dict(login='plenoai'), full_name='plenoai/example',
                                default_branch='main', fork=False),
                workflow_run=dict(event='push', head_branch='main',
                                  head_repository=dict(full_name='plenoai/example'),
                                  conclusion=conclusion, workflow_id=12, head_sha='abc', id=34,
                                  name='CI', html_url='https://github.com/plenoai/example/actions/runs/34'))


class Checks(unittest.TestCase):
    def test_only_owned_default_branch_runs_are_eligible(self):
        original = event()
        self.assertTrue(heal.eligible(original))
        for path, value in [(('repository', 'owner', 'login'), 'other'),
                            (('repository', 'fork'), True),
                            (('workflow_run', 'event'), 'pull_request'),
                            (('workflow_run', 'head_branch'), 'feature'),
                            (('workflow_run', 'head_repository', 'full_name'), 'other/example')]:
            e = copy.deepcopy(original)
            current = e
            for key in path[:-1]:
                current = current[key]
            current[path[-1]] = value
            self.assertFalse(heal.eligible(e))

    def test_failed_run_creates_one_issue_and_requests_repair(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / 'output'
            with patch.dict(os.environ, GITHUB_OUTPUT=str(target)), \
                 patch.object(heal, 'output', side_effect=['[]', 'https://github.com/plenoai/example/issues/9']), \
                 patch.object(heal, 'api', side_effect=[dict(commit=dict(message='ordinary change')), dict(sha='abc')]):
                heal.observe(event())
            self.assertEqual(target.read_text(), 'issue=9\n')

    def test_duplicate_failure_and_repair_commit_do_not_loop(self):
        info = dict(workflow_id=12, sha='abc', run_id=34)
        issue = dict(number=9, body=f'<!-- self-heal:{json.dumps(info)} -->')
        with patch.object(heal, 'output', return_value=json.dumps([issue])) as out:
            heal.observe(event())
            self.assertEqual(out.call_count, 1)
        with tempfile.TemporaryDirectory() as temp, \
             patch.dict(os.environ, GITHUB_OUTPUT=str(Path(temp) / 'output')), \
             patch.object(heal, 'output', side_effect=['[]', 'https://github.com/plenoai/example/issues/9']), \
             patch.object(heal, 'api', side_effect=[dict(commit=dict(message='fix: self-heal #8 (CI)')), dict(sha='abc')]), \
             patch.object(heal, 'comment') as comment:
            heal.observe(event())
            self.assertFalse((Path(temp) / 'output').exists())
            comment.assert_called_once()

    def test_success_only_closes_matching_workflow_on_same_or_newer_sha(self):
        for relation, should_close in [('ahead', True), ('identical', True), ('behind', False), ('diverged', False)]:
            issues = [dict(number=9, body='<!-- self-heal:{"workflow_id":12,"sha":"old"} -->'),
                      dict(number=10, body='<!-- self-heal:{"workflow_id":99,"sha":"old"} -->')]
            with patch.object(heal, 'output', return_value=json.dumps(issues)), \
                 patch.object(heal, 'api', return_value=dict(status=relation)), \
                 patch.object(heal, 'run') as run:
                heal.observe(event('success'))
                self.assertEqual(run.called, should_close)
                if should_close:
                    self.assertEqual(run.call_args.args[:4], ('gh', 'issue', 'close', '9'))

    def test_changed_files_keep_tests_automation_dependencies_and_deletions_immutable(self):
        old = Path.cwd()
        with tempfile.TemporaryDirectory() as temp:
            os.chdir(temp)
            try:
                for name in ['src/main.py', 'tests/main.py', '.github/workflows/ci.yml',
                             'src/main.test.ts', 'package.json', 'setup.py']:
                    p = Path(name)
                    p.parent.mkdir(parents=True, exist_ok=True)
                    p.write_text('pass')
                    with patch.object(heal, 'output', side_effect=[name, '']):
                        if name == 'src/main.py':
                            self.assertEqual(heal.changed_files(), [name])
                        else:
                            with self.assertRaises(ValueError):
                                heal.changed_files()
                Path('src/link.py').symlink_to('main.py')
                for name in ['missing.py', 'src/link.py', '']:
                    with patch.object(heal, 'output', side_effect=[name, '']):
                        with self.assertRaises(ValueError):
                            heal.changed_files()
            finally:
                os.chdir(old)

    def test_checks_must_match_sha_finish_successfully_and_run_jobs(self):
        for conclusion, jobs, succeeds in [('success', ['success', 'skipped'], True),
                                          ('failure', ['failure'], False),
                                          ('success', ['skipped'], False),
                                          ('success', ['success', 'failure'], False)]:
            runs = dict(workflow_runs=[dict(head_sha='new', status='completed', conclusion=conclusion,
                                           id=5, html_url='https://example.com/run/5')])
            with patch.object(heal, 'run'), patch.object(heal, 'api', side_effect=[runs, dict(jobs=[dict(conclusion=j) for j in jobs])]):
                if succeeds:
                    self.assertEqual(heal.wait_for_checks('plenoai/example', 'ci.yml', 'repair', 'new'), 'https://example.com/run/5')
                else:
                    with self.assertRaises(ValueError):
                        heal.wait_for_checks('plenoai/example', 'ci.yml', 'repair', 'new')

    def test_repair_merges_only_after_checks_then_dispatches_new_main(self):
        calls = []
        def out(*args):
            calls.append(args)
            if args[:3] == ('git', 'rev-parse', 'HEAD'):
                return 'new' if any(c[:2] == ('git', '-c') and 'commit' in c for c in calls) else 'abc'
            if args[:3] == ('gh', 'run', 'view'): return 'failure log'
            if args[:3] == ('gh', 'pr', 'create'): return 'https://example.com/pr/1'
            raise AssertionError(args)
        def execute(*args, **kwargs):
            calls.append(args)
            return type('Result', (), {'stdout': 'fixed'})()
        env = dict(FAILURE_ISSUE='9', CHECK_WORKFLOWS='["ci.yml"]', POST_MERGE_WORKFLOWS='["deploy.yml"]')
        with patch.dict(os.environ, env), patch.object(heal, 'output', side_effect=out), \
             patch.object(heal, 'run', side_effect=execute), patch.object(heal, 'changed_files', return_value=['src/main.py']), \
             patch.object(heal, 'api', return_value=dict(sha='abc')), \
             patch.object(heal, 'wait_for_checks', return_value='checked') as checks:
            heal.repair(event())
            checks.assert_called_once_with('plenoai/example', 'ci.yml', 'self-heal/34', 'new')
        merge = next(i for i, c in enumerate(calls) if c[:3] == ('gh', 'pr', 'merge'))
        deploy = next(i for i, c in enumerate(calls) if c[:3] == ('gh', 'workflow', 'run'))
        self.assertLess(merge, deploy)
        self.assertIn('--match-head-commit', calls[merge])
        self.assertEqual(calls[deploy][-2:], ('--ref', 'main'))
        calls.clear()
        with patch.dict(os.environ, env), patch.object(heal, 'output', side_effect=out), \
             patch.object(heal, 'run', side_effect=execute), patch.object(heal, 'changed_files', return_value=['src/main.py']), \
             patch.object(heal, 'wait_for_checks', side_effect=ValueError('failed')):
            with self.assertRaises(ValueError):
                heal.repair(event())
        self.assertFalse(any(c[:3] == ('gh', 'pr', 'merge') for c in calls))
        self.assertFalse(any(c[:3] == ('gh', 'workflow', 'run') for c in calls))


if __name__ == '__main__':
    unittest.main()
