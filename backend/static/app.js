import Surreal from '/scripts/surrealdb.js';

console.log("Connecting to SurrealDB...");
const db = new Surreal('http://127.0.0.1:8000/rpc');
console.log("Connected!");

async function main() {

	try {

		// Signin to a scope from the browser
		await db.signin({
			NS: 'test',
			DB: 'test',
			SC: 'user',
			user: 'info@surrealdb.com',
			pass: 'my-secret-password',
		});

		// Select a specific namespace / database
		await db.use('test', 'test');

		// Create a new person with a random id
		let created = await db.create("person", {
			title: 'Founder & CEO',
			name: {
				first: 'Tobie',
				last: 'Morgan Hitchcock',
			},
			marketing: true,
			identifier: Math.random().toString(36).substr(2, 10),
		});
        console.log(created);

		// Update a person record with a specific id
		let updated = await db.change("person:jaime", {
			marketing: true,
		});
        console.log(updated);

		// Select all people records
		let people = await db.select("person");
        console.log(people);

		// Perform a custom advanced query
		let groups = await db.query('SELECT marketing, count() FROM type::table($tb) GROUP BY marketing', {
			tb: 'person',
		});
        console.log(groups);

	} catch (e) {

		console.error('ERROR', e);

	}

}

main();