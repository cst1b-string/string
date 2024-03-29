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
				darkNavbar: "#002266",
				darkForm: "#002266",
				darkInput: "#1e306b",
				darkSelected: "#555E9F",
				darkHover: "#1e306b",
				darkCircularChatButton: "#7C83C8",
				darkBackground: "#1e306b",
				darkText: "#f8f8ff",
				darkSidebar: "#001C52",
				darkNewChat: "#001133",
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
	darkMode: 'selector',
	plugins: [],
};
export default config;
