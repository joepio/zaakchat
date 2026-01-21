import React, { createContext, useContext, useEffect, useState } from 'react';

export type ThemeId = 'rijksoverheid' | 'utrecht';
export type ColorMode = 'auto' | 'light' | 'dark';

interface Theme {
  id: ThemeId;
  name: string;
  variables: Record<string, string>;
}

const themes: Record<ThemeId, Theme> = {
  rijksoverheid: {
    id: 'rijksoverheid',
    name: 'Rijksoverheid',
    variables: {
      '--body-font': '"RijksoverheidSans", Arial, sans-serif',
      '--header-font': '"RijksoverheidSans", Arial, sans-serif',
      '--color-primary': '#007bc7',
      '--color-primary-dark': '#01689b',
      '--color-secondary': '#42145f',
      '--color-secondary-dark': '#42145f',
      '--border-radius': '5px',
    },
  },
  utrecht: {
    id: 'utrecht',
    name: 'Utrecht',
    variables: {
      '--body-font': '"Open Sans", sans-serif',
      '--header-font': '"Lucida Grande", sans-serif',
      '--color-primary': '#cc0100',
      '--color-primary-dark': '#cc0100',
      '--color-secondary': '#24578f',
      '--color-secondary-dark': '#24578f',
      '--border-radius': '0px',
    },
  },
};

interface ThemeContextType {
  themeId: ThemeId;
  setTheme: (id: ThemeId) => void;
  colorMode: ColorMode;
  setColorMode: (mode: ColorMode) => void;
  availableThemes: Theme[];
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [themeId, setThemeId] = useState<ThemeId>(() => {
    return (localStorage.getItem('theme-preference') as ThemeId) || 'rijksoverheid';
  });

  const [colorMode, setColorMode] = useState<ColorMode>(() => {
    return (localStorage.getItem('color-mode-preference') as ColorMode) || 'auto';
  });

  useEffect(() => {
    const theme = themes[themeId];
    if (theme) {
      const root = document.documentElement;
      Object.entries(theme.variables).forEach(([key, value]) => {
        root.style.setProperty(key, value);
      });
      localStorage.setItem('theme-preference', themeId);
    }
  }, [themeId]);

  useEffect(() => {
    const root = document.documentElement;
    root.classList.remove('light-theme', 'dark-theme');

    if (colorMode === 'dark') {
      root.classList.add('dark-theme');
    } else if (colorMode === 'light') {
      root.classList.add('light-theme');
    }
    // 'auto' does nothing here, relying on media queries

    localStorage.setItem('color-mode-preference', colorMode);
  }, [colorMode]);

  const availableThemes = Object.values(themes);

  return (
    <ThemeContext.Provider value={{
      themeId,
      setTheme: setThemeId,
      colorMode,
      setColorMode,
      availableThemes
    }}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};
