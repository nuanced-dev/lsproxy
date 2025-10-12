import React, { useState, useEffect } from 'react';
import { Node } from './node';
import { AStar } from './astar';
import { Grid } from './types';
import { GridDisplay } from './GridDisplay';
import { Controls } from './Controls';
import { InfoPanel } from './InfoPanel';

interface PathfinderDisplayProps {
    initialMaze?: Grid;
    start?: { x: number; y: number };
    end?: { x: number; y: number };
    allowDiagonal?: boolean;
}

export const PathfinderDisplay: React.FC<PathfinderDisplayProps> = ({
    initialMaze = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 0, 0, 100, 0, 0],
        [0, 0, 100, 100, 100, 100, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ],
    start = { x: 0, y: 0 },
    end = { x: 7, y: 7 },
    allowDiagonal = true,
}) => {
    const [maze, setMaze] = useState<Grid>(initialMaze);
    const [path, setPath] = useState<Node[] | null>(null);
    const [isAnimating, setIsAnimating] = useState(false);
    const [currentStep, setCurrentStep] = useState(0);

    const findPath = () => {
        const astar = new AStar(maze, start.x, start.y, allowDiagonal);
        const newPath = astar.findPathTo(end.x, end.y);
        setPath(newPath);
        setCurrentStep(0);
        setIsAnimating(true);
    };

    useEffect(() => {
        if (isAnimating && path && currentStep < path.length) {
            const timer = setTimeout(() => {
                setCurrentStep(prev => prev + 1);
            }, 500);
            return () => clearTimeout(timer);
        }
        if (currentStep >= (path?.length ?? 0)) {
            setIsAnimating(false);
        }
    }, [isAnimating, currentStep, path]);

    const toggleCell = (x: number, y: number) => {
        if ((x === start.x && y === start.y) || (x === end.x && y === end.y)) return;
        
        const newMaze = maze.map(row => [...row]);
        newMaze[y][x] = newMaze[y][x] === 100 ? 0 : 100;
        setMaze(newMaze);
        setPath(null);
        setCurrentStep(0);
    };

    const handleReset = () => {
        setMaze(initialMaze);
        setPath(null);
        setCurrentStep(0);
    };

    return (
        <div className="p-4">
            <Controls 
                onFindPath={findPath}
                onReset={handleReset}
                isAnimating={isAnimating}
            />
            
            <GridDisplay 
                maze={maze}
                start={start}
                end={end}
                path={path}
                currentStep={currentStep}
                isAnimating={isAnimating}
                onCellClick={toggleCell}
            />

            <InfoPanel path={path} />
        </div>
    );
};
