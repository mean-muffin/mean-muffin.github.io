import { readFileSync, existsSync } from 'node:fs';

const required = [
  'astro.config.mjs',
  'src/pages/index.astro',
  'src/pages/privacy.astro',
  'src/pages/security.astro',
  'public/mean-muffin-mark.svg',
  '.github/workflows/deploy-pages.yml',
  'contracts/site.schema.json',
];
const errors = [];
for (const path of required) if (!existsSync(path)) errors.push(`missing ${path}`);
const pkg = JSON.parse(readFileSync('package.json', 'utf8'));
if (pkg.dependencies?.astro !== '7.3.3') errors.push('Astro must be pinned to 7.3.3');
const workflow = readFileSync('.github/workflows/deploy-pages.yml', 'utf8');
for (const sha of ['3d3c42e5aac5ba805825da76410c181273ba90b1','49933ea5288caeca8642d1e84afbd3f7d6820020','983d7736d9b0ae728b81ab479565c72886d7745b','56afc609e74202658d3ffba0e8f6dda462b719fa','d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e']) if (!workflow.includes(sha)) errors.push(`workflow action is not pinned: ${sha}`);
if (errors.length) { errors.forEach((e) => console.error(`conformance: ${e}`)); process.exit(1); }
console.log('Mean Muffin marketing-site conformance: PASS');
