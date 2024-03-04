import type { Config } from "tailwindcss";

const config: Config = {
	content: [
		"./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
		"./src/components/**/*.{js,ts,jsx,tsx,mdx}",
		"./src/app/**/*.{js,ts,jsx,tsx,mdx}",
	],
	theme: {
		extend: {
			colors: {
				// 383a40
				lightGrey: "#383838",
				hoverLightGrey: "#515151",
				darkGrey: "#16181b",
				discordGreen: "#23a559",
				navbarGrey: "#202327",
				navbarBlue: "#191970",
				formBlue: "#113355",
				buttonBlue: "#335577",
				hoverBlue: "#224466",
			},
			height: {
				"90p": "90%",
			},
			width: {
				"90p": "90%",
			},
			inset: {
				"2.5p": "2.5%",
			},
			backgroundImage: {
				"gradient-radial": "radial-gradient(var(--tw-gradient-stops))",
				"gradient-conic": "conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))",
			},
			borderWidth: {
				DEFAULT: "1px",
				0: "0",
				2: "2px",
				4: "4px",
				8: "8px",
			},
		},
	},
	plugins: [],
};
export default config;
