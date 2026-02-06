import { Box, Text, Group, ActionIcon, Collapse } from '@mantine/core';
import { IconChevronRight, IconChevronDown, IconDatabase, IconTable, IconFile, IconFileText, IconTrash } from '@tabler/icons-react';
import { useState } from 'react';
import type { TreeNode } from '../types/index';

interface TreeNodeItemProps {
  node: TreeNode;
  level: number;
  onSelect: (node: TreeNode) => void;
  onExpand: (node: TreeNode) => void;
  onDelete?: (node: TreeNode) => void;
  selectedId?: string;
}

const getIcon = (type: TreeNode['type']) => {
  switch (type) {
    case 'database':
      return <IconDatabase size={16} />;
    case 'table':
      return <IconTable size={16} />;
    case 'document':
      return <IconFile size={16} />;
    case 'child':
      return <IconFileText size={16} />;
  }
};

export function TreeNodeItem({ node, level, onSelect, onExpand, onDelete, selectedId }: TreeNodeItemProps) {
  const [expanded, setExpanded] = useState(false);
  const [isHovered, setIsHovered] = useState(false);

  const handleToggle = () => {
    if (node.hasChildren) {
      setExpanded(!expanded);
      if (!expanded && (!node.children || node.children.length === 0)) {
        onExpand(node);
      }
    }
  };

  const handleClick = () => {
    onSelect(node);
  };

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (onDelete) {
      onDelete(node);
    }
  };

  const isSelected = selectedId === node.id;
  const canDelete = node.type === 'database' || node.type === 'table' || node.type === 'document';

  return (
    <Box>
      <Group
        gap="xs"
        style={{
          paddingLeft: level * 20,
          paddingTop: 4,
          paddingBottom: 4,
          cursor: 'pointer',
          backgroundColor: isSelected ? '#e7f5ff' : 'transparent',
        }}
        onMouseEnter={(e) => {
          setIsHovered(true);
          if (!isSelected) {
            e.currentTarget.style.backgroundColor = '#f8f9fa';
          }
        }}
        onMouseLeave={(e) => {
          setIsHovered(false);
          if (!isSelected) {
            e.currentTarget.style.backgroundColor = 'transparent';
          }
        }}
      >
        {node.hasChildren ? (
          <ActionIcon
            variant="subtle"
            size="xs"
            onClick={handleToggle}
          >
            {expanded ? <IconChevronDown size={14} /> : <IconChevronRight size={14} />}
          </ActionIcon>
        ) : (
          <Box style={{ width: 22 }} />
        )}
        <Group gap={6} onClick={handleClick} style={{ flex: 1 }}>
          {getIcon(node.type)}
          <Text size="sm">{node.label}</Text>
        </Group>
        {canDelete && isHovered && (
          <ActionIcon
            variant="subtle"
            size="xs"
            color="red"
            onClick={handleDelete}
            title={`Delete ${node.type}`}
          >
            <IconTrash size={14} />
          </ActionIcon>
        )}
      </Group>
      
      {node.hasChildren && expanded && node.children && (
        <Collapse in={expanded}>
          {node.children.map(child => (
            <TreeNodeItem
              key={child.id}
              node={child}
              level={level + 1}
              onSelect={onSelect}
              onExpand={onExpand}
              onDelete={onDelete}
              selectedId={selectedId}
            />
          ))}
        </Collapse>
      )}
    </Box>
  );
}
