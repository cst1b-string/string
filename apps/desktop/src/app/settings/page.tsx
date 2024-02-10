// Profile display name, bio?, dark/light mode, clear chats, etc.
'use client'
import { useState } from 'react';

// var lightMode = true;

// function toggle_light_mode() {
	
// }

export default function Home() {

	const [lightMode, setLightMode] = useState(true);

	if (!lightMode) {
		return (
			<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

				<button onClick={() => setLightMode(!lightMode)}>
					<h1> Light Mode </h1>
				</button>

			</div>
		);
	}

	else {
		return (
			<div className="flex justify-center bg-[#3399FF] text-[black] px-4 py-2 rounded-md">

				<button onClick={() => setLightMode(!lightMode)}>
					<h1> Dark Mode </h1>
				</button>

			</div>
		);
	}
}
