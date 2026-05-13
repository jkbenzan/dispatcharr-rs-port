const fs = require('fs');
let content = fs.readFileSync('svelte-frontend/src/routes/stream-checker/+page.svelte', 'utf8');

// The faulty line
const badStr = "toast.info(Sorting streams in  channels...);";
const goodStr = "toast.info(Sorting streams in  channels...);";

if (content.includes(badStr)) {
	content = content.replace(badStr, goodStr);
	fs.writeFileSync('svelte-frontend/src/routes/stream-checker/+page.svelte', content, 'utf8');
	console.log("Fixed toast.info syntax error.");
} else {
	console.log("Could not find the faulty line.");
}
