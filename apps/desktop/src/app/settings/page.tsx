// Profile display name, bio?, dark/light mode, clear chats, etc.
'use client'
import { useContext } from 'react';
import { themeContext } from '../page';

// var lightMode = true;

// function toggle_light_mode() {
	
// }

export default function Home() {

	const lightModeComposite = useContext(themeContext);
	const lightMode = lightModeComposite.lightMode;
	var lightModeText;
	if (lightMode) {
		lightModeText = "Dark Mode"
	}

	else {
		lightModeText = "Light Mode"
	}
	return (
		<div className="py-5 px-4">
			<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md size-sm">

				<button onClick={() => lightModeComposite.setLightMode(true)}>
					<h1> {lightModeText} </h1>
				</button>

			</div>

			<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

				<button onClick={() => false}>
					<h1> Setting 2 </h1>
				</button>
			</div>

			<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

				<button onClick={() => false}>
					<h1> Setting 3 </h1>
				</button>

			</div>

		</div>
	);
}
