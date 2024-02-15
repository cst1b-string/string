// Profile display name, bio?, dark/light mode, clear chats, etc.
'use client'
import { useContext } from 'react';
import { themeContext } from '../page';

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
		<div className="flex justify-center">
			<div className="py-5 w-1/3 space-y-1">
				<div className="flex justify-center bg-[#113355] text-[white] px-4 py-2 rounded-md">

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
		</div>
	);
}
