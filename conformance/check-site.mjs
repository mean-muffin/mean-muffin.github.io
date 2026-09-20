import { readFileSync, existsSync } from 'node:fs';

const required = [
  'astro.config.mjs',
  'src/pages/index.astro',
  'src/pages/privacy.astro',
  'src/pages/security.astro',
  'public/mean-muffin-mark.svg',
  '.github/workflows/deploy-pages.yml',
  'contracts/site.schema.json',
  'package-lock.json',
];
const errors = [];
for (const path of required) if (!existsSync(path)) errors.push(`missing ${path}`);

const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
if (pkg.dependencies?.astro !== '7.3.3') errors.push('Astro must be pinned to 7.3.3');
if (pkg.devDependencies?.['@astrojs/check'] !== '0.9.4') errors.push('@astrojs/check must be pinned to 0.9.4');
if (pkg.devDependencies?.typescript !== '5.9.2') errors.push('TypeScript must be pinned to 5.9.2');

const lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
if (lock.lockfileVersion !== 3) errors.push('npm lockfileVersion must be 3');
const root = lock.packages?.[''] ?? {};
if (root.dependencies?.astro !== pkg.dependencies?.astro) errors.push('Astro root lock pin differs from package.json');
if (root.devDependencies?.['@astrojs/check'] !== pkg.devDependencies?.['@astrojs/check']) errors.push('@astrojs/check root lock pin differs from package.json');
if (root.devDependencies?.typescript !== pkg.devDependencies?.typescript) errors.push('TypeScript root lock pin differs from package.json');
const allowedInstallScripts = new Set(['node_modules/esbuild@0.28.2', 'node_modules/fsevents@2.3.3']);
const actualInstallScripts = new Set(Object.entries(lock.packages ?? {})
  .filter(([, value]) => value.hasInstallScript)
  .map(([name, value]) => `${name}@${value.version}`));
if (actualInstallScripts.size !== allowedInstallScripts.size || [...actualInstallScripts].some((entry) => !allowedInstallScripts.has(entry))) {
  errors.push(`unexpected install-script packages: ${[...actualInstallScripts].join(', ')}`);
}

const workflow = readFileSync('.github/workflows/deploy-pages.yml', 'utf8');
for (const sha of ['3d3c42e5aac5ba805825da76410c181273ba90b1','49933ea5288caeca8642d1e84afbd3f7d6820020','983d7736d9b0ae728b81ab479565c72886d7745b','56afc609e74202658d3ffba0e8f6dda462b719fa','d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e']) {
  if (!workflow.includes(sha)) errors.push(`workflow action is not pinned: ${sha}`);
}
if (!workflow.includes('npm ci --no-audit --no-fund')) errors.push('Pages workflow must use npm ci');
if (!workflow.includes('cache: npm')) errors.push('Pages workflow must cache from the committed npm lock');
if (errors.length) { errors.forEach((e) => console.error(`conformance: ${e}`)); process.exit(1); }
console.log('Mean Muffin marketing-site conformance: PASS');
